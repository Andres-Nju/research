//! The `fullnode` module hosts all the fullnode microservices.

use crate::bank_forks::BankForks;
use crate::blocktree::Blocktree;
use crate::blocktree_processor::{self, BankForksInfo};
use crate::cluster_info::{ClusterInfo, Node};
use crate::contact_info::ContactInfo;
use crate::entry::create_ticks;
use crate::entry::next_entry_mut;
use crate::entry::Entry;
use crate::gossip_service::{discover_nodes, GossipService};
use crate::leader_schedule_utils;
use crate::poh_recorder::PohRecorder;
use crate::poh_service::{PohService, PohServiceConfig};
use crate::rpc::JsonRpcConfig;
use crate::rpc_pubsub_service::PubSubService;
use crate::rpc_service::JsonRpcService;
use crate::rpc_subscriptions::RpcSubscriptions;
use crate::service::Service;
use crate::storage_stage::StorageState;
use crate::tpu::Tpu;
use crate::tvu::{Sockets, Tvu};
use solana_metrics::counter::Counter;
use solana_sdk::genesis_block::GenesisBlock;
use solana_sdk::hash::Hash;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, KeypairUtil};
use solana_sdk::system_transaction;
use solana_sdk::timing::timestamp;
use solana_sdk::transaction::Transaction;
use solana_vote_api::vote_instruction;
use solana_vote_api::vote_state::Vote;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex, RwLock};
use std::thread::Result;

#[derive(Clone, Debug)]
pub struct FullnodeConfig {
    pub sigverify_disabled: bool,
    pub voting_disabled: bool,
    pub blockstream: Option<String>,
    pub storage_rotate_count: u64,
    pub tick_config: PohServiceConfig,
    pub account_paths: Option<String>,
    pub rpc_config: JsonRpcConfig,
}
impl Default for FullnodeConfig {
    fn default() -> Self {
        // TODO: remove this, temporary parameter to configure
        // storage amount differently for test configurations
        // so tests don't take forever to run.
        const NUM_HASHES_FOR_STORAGE_ROTATE: u64 = 1024;
        Self {
            sigverify_disabled: false,
            voting_disabled: false,
            blockstream: None,
            storage_rotate_count: NUM_HASHES_FOR_STORAGE_ROTATE,
            tick_config: PohServiceConfig::default(),
            account_paths: None,
            rpc_config: JsonRpcConfig::default(),
        }
    }
}

pub struct Fullnode {
    pub id: Pubkey,
    exit: Arc<AtomicBool>,
    rpc_service: Option<JsonRpcService>,
    rpc_pubsub_service: Option<PubSubService>,
    gossip_service: GossipService,
    poh_recorder: Arc<Mutex<PohRecorder>>,
    poh_service: PohService,
    tpu: Tpu,
    tvu: Tvu,
}

impl Fullnode {
    pub fn new<T>(
        mut node: Node,
        keypair: &Arc<Keypair>,
        ledger_path: &str,
        vote_account: &Pubkey,
        voting_keypair: T,
        entrypoint_info_option: Option<&ContactInfo>,
        config: &FullnodeConfig,
    ) -> Self
    where
        T: 'static + KeypairUtil + Sync + Send,
    {
        info!("creating bank...");

        let id = keypair.pubkey();
        assert_eq!(id, node.info.id);

        let (bank_forks, bank_forks_info, blocktree, ledger_signal_receiver) =
            new_banks_from_blocktree(ledger_path, config.account_paths.clone());

        let exit = Arc::new(AtomicBool::new(false));
        let bank_info = &bank_forks_info[0];
        let bank = bank_forks[bank_info.bank_slot].clone();

        info!(
            "starting PoH... {} {}",
            bank.tick_height(),
            bank.last_blockhash(),
        );
        let blocktree = Arc::new(blocktree);
        let (poh_recorder, entry_receiver) = PohRecorder::new_with_clear_signal(
            bank.tick_height(),
            bank.last_blockhash(),
            bank.slot(),
            leader_schedule_utils::next_leader_slot(&id, bank.slot(), &bank, Some(&blocktree)),
            bank.ticks_per_slot(),
            &id,
            &blocktree,
            blocktree.new_blobs_signals.first().cloned(),
        );
        let poh_recorder = Arc::new(Mutex::new(poh_recorder));
        let poh_service = PohService::new(poh_recorder.clone(), &config.tick_config, &exit);
        assert_eq!(
            blocktree.new_blobs_signals.len(),
            1,
            "New blob signal for the TVU should be the same as the clear bank signal."
        );

        info!("node info: {:?}", node.info);
        info!("node entrypoint_info: {:?}", entrypoint_info_option);
        info!(
            "node local gossip address: {}",
            node.sockets.gossip.local_addr().unwrap()
        );

        let bank_forks = Arc::new(RwLock::new(bank_forks));

        node.info.wallclock = timestamp();
        let cluster_info = Arc::new(RwLock::new(ClusterInfo::new(
            node.info.clone(),
            keypair.clone(),
        )));

        let storage_state = StorageState::new();

        let rpc_service = if node.info.rpc.port() == 0 {
            None
        } else {
            Some(JsonRpcService::new(
                &cluster_info,
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), node.info.rpc.port()),
                storage_state.clone(),
                config.rpc_config.clone(),
                bank_forks.clone(),
                &exit,
            ))
        };

        let subscriptions = Arc::new(RpcSubscriptions::default());
        let rpc_pubsub_service = if node.info.rpc_pubsub.port() == 0 {
            None
        } else {
            Some(PubSubService::new(
                &subscriptions,
                SocketAddr::new(
                    IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
                    node.info.rpc_pubsub.port(),
                ),
                &exit,
            ))
        };

        let gossip_service = GossipService::new(
            &cluster_info,
            Some(blocktree.clone()),
            Some(bank_forks.clone()),
            node.sockets.gossip,
            &exit,
        );

        // Insert the entrypoint info, should only be None if this node
        // is the bootstrap leader

        if let Some(entrypoint_info) = entrypoint_info_option {
            cluster_info
                .write()
                .unwrap()
                .set_entrypoint(entrypoint_info.clone());
        }

        let sockets = Sockets {
            repair: node
                .sockets
                .repair
                .try_clone()
                .expect("Failed to clone repair socket"),
            retransmit: node
                .sockets
                .retransmit
                .try_clone()
                .expect("Failed to clone retransmit socket"),
            fetch: node
                .sockets
                .tvu
                .iter()
                .map(|s| s.try_clone().expect("Failed to clone TVU Sockets"))
                .collect(),
        };

        let voting_keypair = if config.voting_disabled {
            None
        } else {
            Some(Arc::new(voting_keypair))
        };

        // Setup channel for sending entries to storage stage
        let (sender, receiver) = channel();

        let tvu = Tvu::new(
            vote_account,
            voting_keypair,
            &bank_forks,
            &bank_forks_info,
            &cluster_info,
            sockets,
            blocktree.clone(),
            config.storage_rotate_count,
            &storage_state,
            config.blockstream.as_ref(),
            ledger_signal_receiver,
            &subscriptions,
            &poh_recorder,
            sender.clone(),
            receiver,
            &exit,
        );
        let tpu = Tpu::new(
            &id,
            &cluster_info,
            &poh_recorder,
            entry_receiver,
            node.sockets.tpu,
            node.sockets.tpu_via_blobs,
            node.sockets.broadcast,
            config.sigverify_disabled,
            &blocktree,
            sender,
            &exit,
        );

        inc_new_counter_info!("fullnode-new", 1);
        Self {
            id,
            gossip_service,
            rpc_service,
            rpc_pubsub_service,
            tpu,
            tvu,
            exit,
            poh_service,
            poh_recorder,
        }
    }

    // Used for notifying many nodes in parallel to exit
    pub fn exit(&self) {
        self.exit.store(true, Ordering::Relaxed);
    }

    pub fn close(self) -> Result<()> {
        self.exit();
        self.join()
    }
}

pub fn new_banks_from_blocktree(
    blocktree_path: &str,
    account_paths: Option<String>,
) -> (BankForks, Vec<BankForksInfo>, Blocktree, Receiver<bool>) {
    let genesis_block =
        GenesisBlock::load(blocktree_path).expect("Expected to successfully open genesis block");

    let (blocktree, ledger_signal_receiver) = Blocktree::open_with_signal(blocktree_path)
        .expect("Expected to successfully open database ledger");

    let (bank_forks, bank_forks_info) =
        blocktree_processor::process_blocktree(&genesis_block, &blocktree, account_paths)
            .expect("process_blocktree failed");

    (
        bank_forks,
        bank_forks_info,
        blocktree,
        ledger_signal_receiver,
    )
}

impl Service for Fullnode {
    type JoinReturnType = ();

    fn join(self) -> Result<()> {
        self.poh_service.join()?;
        drop(self.poh_recorder);
        if let Some(rpc_service) = self.rpc_service {
            rpc_service.join()?;
        }
        if let Some(rpc_pubsub_service) = self.rpc_pubsub_service {
            rpc_pubsub_service.join()?;
        }

        self.gossip_service.join()?;
        self.tpu.join()?;
        self.tvu.join()?;

        Ok(())
    }
}

// Create entries such the node identified by active_keypair will be added to the active set for
// leader selection, and append `num_ending_ticks` empty tick entries.
pub fn make_active_set_entries(
    active_keypair: &Arc<Keypair>,
    lamport_source: &Keypair,
    stake: u64,
    slot_to_vote_on: u64,
    blockhash: &Hash,
    num_ending_ticks: u64,
) -> (Vec<Entry>, Keypair) {
    // 1) Assume the active_keypair node has no lamports staked
    let transfer_tx = system_transaction::create_user_account(
        &lamport_source,
        &active_keypair.pubkey(),
        stake,
        *blockhash,
        0,
    );
    let mut last_entry_hash = *blockhash;
    let transfer_entry = next_entry_mut(&mut last_entry_hash, 1, vec![transfer_tx]);

    // 2) Create and register a vote account for active_keypair
    let voting_keypair = Keypair::new();
    let vote_account_id = voting_keypair.pubkey();

    let new_vote_account_ixs = vote_instruction::create_account(
        &active_keypair.pubkey(),
        &vote_account_id,
        &active_keypair.pubkey(),
        0,
        stake.saturating_sub(2),
    );
    let new_vote_account_tx = Transaction::new_signed_instructions(
        &[active_keypair.as_ref()],
        new_vote_account_ixs,
        *blockhash,
    );
    let new_vote_account_entry = next_entry_mut(&mut last_entry_hash, 1, vec![new_vote_account_tx]);

    // 3) Create vote entry
    let vote_ix =
        vote_instruction::vote(&voting_keypair.pubkey(), vec![Vote::new(slot_to_vote_on)]);
    let vote_tx =
        Transaction::new_signed_instructions(&[&voting_keypair], vec![vote_ix], *blockhash);
    let vote_entry = next_entry_mut(&mut last_entry_hash, 1, vec![vote_tx]);

    // 4) Create `num_ending_ticks` empty ticks
    let mut entries = vec![transfer_entry, new_vote_account_entry, vote_entry];
    let empty_ticks = create_ticks(num_ending_ticks, last_entry_hash);
    entries.extend(empty_ticks);

    (entries, voting_keypair)
}

pub fn new_fullnode_for_tests() -> (Fullnode, ContactInfo, Keypair, String) {
    use crate::blocktree::create_new_tmp_ledger;
    use crate::cluster_info::Node;

    let node_keypair = Arc::new(Keypair::new());
    let node = Node::new_localhost_with_pubkey(&node_keypair.pubkey());
    let contact_info = node.info.clone();

    let (mut genesis_block, mint_keypair) =
        GenesisBlock::new_with_leader(10_000, &contact_info.id, 42);
    genesis_block
        .native_instruction_processors
        .push(("solana_budget_program".to_string(), solana_budget_api::id()));

    let (ledger_path, _blockhash) = create_new_tmp_ledger!(&genesis_block);

    let voting_keypair = Keypair::new();
    let node = Fullnode::new(
        node,
        &node_keypair,
        &ledger_path,
        &voting_keypair.pubkey(),
        voting_keypair,
        None,
        &FullnodeConfig::default(),
    );
    discover_nodes(&contact_info.gossip, 1).expect("Node startup failed");
    (node, contact_info, mint_keypair, ledger_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blocktree::create_new_tmp_ledger;
    use std::fs::remove_dir_all;

    #[test]
    fn validator_exit() {
        let leader_keypair = Keypair::new();
        let leader_node = Node::new_localhost_with_pubkey(&leader_keypair.pubkey());

        let validator_keypair = Keypair::new();
        let validator_node = Node::new_localhost_with_pubkey(&validator_keypair.pubkey());
        let (genesis_block, _mint_keypair) =
            GenesisBlock::new_with_leader(10_000, &leader_keypair.pubkey(), 1000);
        let (validator_ledger_path, _blockhash) = create_new_tmp_ledger!(&genesis_block);

        let voting_keypair = Keypair::new();
        let validator = Fullnode::new(
            validator_node,
            &Arc::new(validator_keypair),
            &validator_ledger_path,
            &voting_keypair.pubkey(),
            voting_keypair,
            Some(&leader_node.info),
            &FullnodeConfig::default(),
        );
        validator.close().unwrap();
        remove_dir_all(validator_ledger_path).unwrap();
    }

    #[test]
    fn validator_parallel_exit() {
        let leader_keypair = Keypair::new();
        let leader_node = Node::new_localhost_with_pubkey(&leader_keypair.pubkey());

        let mut ledger_paths = vec![];
        let validators: Vec<Fullnode> = (0..2)
            .map(|_| {
                let validator_keypair = Keypair::new();
                let validator_node = Node::new_localhost_with_pubkey(&validator_keypair.pubkey());
                let (genesis_block, _mint_keypair) =
                    GenesisBlock::new_with_leader(10_000, &leader_keypair.pubkey(), 1000);
                let (validator_ledger_path, _blockhash) = create_new_tmp_ledger!(&genesis_block);
                ledger_paths.push(validator_ledger_path.clone());
                let voting_keypair = Keypair::new();
                Fullnode::new(
                    validator_node,
                    &Arc::new(validator_keypair),
                    &validator_ledger_path,
                    &voting_keypair.pubkey(),
                    voting_keypair,
                    Some(&leader_node.info),
                    &FullnodeConfig::default(),
                )
            })
            .collect();

        // Each validator can exit in parallel to speed many sequential calls to `join`
        validators.iter().for_each(|v| v.exit());
        // While join is called sequentially, the above exit call notified all the
        // validators to exit from all their threads
        validators.into_iter().for_each(|validator| {
            validator.join().unwrap();
        });

        for path in ledger_paths {
            remove_dir_all(path).unwrap();
        }
    }
}
