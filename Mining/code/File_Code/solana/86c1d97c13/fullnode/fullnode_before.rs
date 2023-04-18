//! The `fullnode` module hosts all the fullnode microservices.

use bank::Bank;
use broadcast_stage::BroadcastStage;
use crdt::{Crdt, NodeInfo, TestNode};
use entry::Entry;
use ledger::read_ledger;
use ncp::Ncp;
use packet::BlobRecycler;
use rpc::{JsonRpcService, RPC_PORT};
use rpu::Rpu;
use service::Service;
use signature::{Keypair, KeypairUtil};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread::{JoinHandle, Result};
use tpu::Tpu;
use tvu::Tvu;
use untrusted::Input;
use window;

pub struct Fullnode {
    exit: Arc<AtomicBool>,
    thread_hdls: Vec<JoinHandle<()>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
/// Fullnode configuration to be stored in file
pub struct Config {
    pub node_info: NodeInfo,
    pkcs8: Vec<u8>,
}

/// Structure to be replicated by the network
impl Config {
    pub fn new(bind_addr: &SocketAddr, pkcs8: Vec<u8>) -> Self {
        let keypair =
            Keypair::from_pkcs8(Input::from(&pkcs8)).expect("from_pkcs8 in fullnode::Config new");
        let pubkey = keypair.pubkey();
        let node_info = NodeInfo::new_leader_with_pubkey(pubkey, bind_addr);
        Config { node_info, pkcs8 }
    }
    pub fn keypair(&self) -> Keypair {
        Keypair::from_pkcs8(Input::from(&self.pkcs8))
            .expect("from_pkcs8 in fullnode::Config keypair")
    }
}

impl Fullnode {
    fn new_internal(
        mut node: TestNode,
        leader: bool,
        ledger_path: &str,
        keypair: Keypair,
        network_entry_for_validator: Option<SocketAddr>,
        sigverify_disabled: bool,
    ) -> Self {
        info!("creating bank...");
        let bank = Bank::new_default(leader);

        let entries = read_ledger(ledger_path, true).expect("opening ledger");

        let entries = entries.map(|e| e.expect("failed to parse entry"));

        info!("processing ledger...");
        let (entry_height, ledger_tail) = bank.process_ledger(entries).expect("process_ledger");
        // entry_height is the network-wide agreed height of the ledger.
        //  initialize it from the input ledger
        info!("processed {} ledger...", entry_height);

        info!("creating networking stack...");

        let local_gossip_addr = node.sockets.gossip.local_addr().unwrap();
        let local_requests_addr = node.sockets.requests.local_addr().unwrap();
        info!(
            "starting... local gossip address: {} (advertising {})",
            local_gossip_addr, node.data.contact_info.ncp
        );
        let requests_addr = node.data.contact_info.rpu;
        let exit = Arc::new(AtomicBool::new(false));
        if !leader {
            let testnet_addr = network_entry_for_validator.expect("validator requires entry");

            let network_entry_point = NodeInfo::new_entry_point(testnet_addr);
            let server = Self::new_validator(
                keypair,
                bank,
                entry_height,
                &ledger_tail,
                node,
                &network_entry_point,
                exit.clone(),
                Some(ledger_path),
                sigverify_disabled,
            );
            info!(
                "validator ready... local request address: {} (advertising {}) connected to: {}",
                local_requests_addr, requests_addr, testnet_addr
            );
            server
        } else {
            node.data.leader_id = node.data.id;

            let server = Self::new_leader(
                keypair,
                bank,
                entry_height,
                &ledger_tail,
                node,
                exit.clone(),
                ledger_path,
                sigverify_disabled,
            );
            info!(
                "leader ready... local request address: {} (advertising {})",
                local_requests_addr, requests_addr
            );
            server
        }
    }

    pub fn new(
        node: TestNode,
        leader: bool,
        ledger: &str,
        keypair: Keypair,
        network_entry_for_validator: Option<SocketAddr>,
    ) -> Self {
        Self::new_internal(
            node,
            leader,
            ledger,
            keypair,
            network_entry_for_validator,
            false,
        )
    }

    pub fn new_without_sigverify(
        node: TestNode,
        leader: bool,
        ledger_path: &str,
        keypair: Keypair,
        network_entry_for_validator: Option<SocketAddr>,
    ) -> Self {
        Self::new_internal(
            node,
            leader,
            ledger_path,
            keypair,
            network_entry_for_validator,
            true,
        )
    }

    /// Create a server instance acting as a leader.
    ///
    /// ```text
    ///              .---------------------.
    ///              |  Leader             |
    ///              |                     |
    ///  .--------.  |  .-----.            |
    ///  |        |---->|     |            |
    ///  | Client |  |  | RPU |            |
    ///  |        |<----|     |            |
    ///  `----+---`  |  `-----`            |
    ///       |      |     ^               |
    ///       |      |     |               |
    ///       |      |  .--+---.           |
    ///       |      |  | Bank |           |
    ///       |      |  `------`           |
    ///       |      |     ^               |
    ///       |      |     |               |    .------------.
    ///       |      |  .--+--.   .-----.  |    |            |
    ///       `-------->| TPU +-->| NCP +------>| Validators |
    ///              |  `-----`   `-----`  |    |            |
    ///              |                     |    `------------`
    ///              `---------------------`
    /// ```
    pub fn new_leader(
        keypair: Keypair,
        bank: Bank,
        entry_height: u64,
        ledger_tail: &[Entry],
        node: TestNode,
        exit: Arc<AtomicBool>,
        ledger_path: &str,
        sigverify_disabled: bool,
    ) -> Self {
        let tick_duration = None;
        // TODO: To light up PoH, uncomment the following line:
        //let tick_duration = Some(Duration::from_millis(1000));

        let bank = Arc::new(bank);
        let mut thread_hdls = vec![];
        let rpu = Rpu::new(
            &bank,
            node.sockets.requests,
            node.sockets.respond,
            exit.clone(),
        );
        thread_hdls.extend(rpu.thread_hdls());

        let rpc_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), RPC_PORT);
        let rpc_service = JsonRpcService::new(bank.clone(), rpc_addr, exit.clone());
        thread_hdls.extend(rpc_service.thread_hdls());

        let blob_recycler = BlobRecycler::default();
        let window =
            window::new_window_from_entries(ledger_tail, entry_height, &node.data, &blob_recycler);

        let crdt = Arc::new(RwLock::new(Crdt::new(node.data).expect("Crdt::new")));

        let (tpu, blob_receiver) = Tpu::new(
            keypair,
            &bank,
            &crdt,
            tick_duration,
            node.sockets.transaction,
            &blob_recycler,
            exit.clone(),
            ledger_path,
            sigverify_disabled,
        );
        thread_hdls.extend(tpu.thread_hdls());
        let ncp = Ncp::new(
            &crdt,
            window.clone(),
            Some(ledger_path),
            node.sockets.gossip,
            node.sockets.gossip_send,
            exit.clone(),
        ).expect("Ncp::new");
        thread_hdls.extend(ncp.thread_hdls());

        let broadcast_stage = BroadcastStage::new(
            node.sockets.broadcast,
            crdt,
            window,
            entry_height,
            blob_recycler.clone(),
            blob_receiver,
        );
        thread_hdls.extend(broadcast_stage.thread_hdls());

        Fullnode { exit, thread_hdls }
    }

    /// Create a server instance acting as a validator.
    ///
    /// ```text
    ///               .-------------------------------.
    ///               | Validator                     |
    ///               |                               |
    ///   .--------.  |            .-----.            |
    ///   |        |-------------->|     |            |
    ///   | Client |  |            | RPU |            |
    ///   |        |<--------------|     |            |
    ///   `--------`  |            `-----`            |
    ///               |               ^               |
    ///               |               |               |
    ///               |            .--+---.           |
    ///               |            | Bank |           |
    ///               |            `------`           |
    ///               |               ^               |
    ///   .--------.  |               |               |    .------------.
    ///   |        |  |            .--+--.            |    |            |
    ///   | Leader |<------------->| TVU +<--------------->|            |
    ///   |        |  |            `-----`            |    | Validators |
    ///   |        |  |               ^               |    |            |
    ///   |        |  |               |               |    |            |
    ///   |        |  |            .--+--.            |    |            |
    ///   |        |<------------->| NCP +<--------------->|            |
    ///   |        |  |            `-----`            |    |            |
    ///   `--------`  |                               |    `------------`
    ///               `-------------------------------`
    /// ```
    pub fn new_validator(
        keypair: Keypair,
        bank: Bank,
        entry_height: u64,
        ledger_tail: &[Entry],
        node: TestNode,
        entry_point: &NodeInfo,
        exit: Arc<AtomicBool>,
        ledger_path: Option<&str>,
        _sigverify_disabled: bool,
    ) -> Self {
        let bank = Arc::new(bank);
        let mut thread_hdls = vec![];
        let rpu = Rpu::new(
            &bank,
            node.sockets.requests,
            node.sockets.respond,
            exit.clone(),
        );
        thread_hdls.extend(rpu.thread_hdls());

        let mut rpc_addr = node.data.contact_info.ncp;
        rpc_addr.set_port(RPC_PORT);
        let rpc_service = JsonRpcService::new(bank.clone(), rpc_addr, exit.clone());
        thread_hdls.extend(rpc_service.thread_hdls());

        let blob_recycler = BlobRecycler::default();
        let window =
            window::new_window_from_entries(ledger_tail, entry_height, &node.data, &blob_recycler);

        let crdt = Arc::new(RwLock::new(Crdt::new(node.data).expect("Crdt::new")));
        crdt.write()
            .expect("'crdt' write lock before insert() in pub fn replicate")
            .insert(&entry_point);

        let ncp = Ncp::new(
            &crdt,
            window.clone(),
            ledger_path,
            node.sockets.gossip,
            node.sockets.gossip_send,
            exit.clone(),
        ).expect("Ncp::new");

        let tvu = Tvu::new(
            keypair,
            &bank,
            entry_height,
            crdt.clone(),
            window.clone(),
            node.sockets.replicate,
            node.sockets.repair,
            node.sockets.retransmit,
            ledger_path,
            exit.clone(),
        );
        thread_hdls.extend(tvu.thread_hdls());
        thread_hdls.extend(ncp.thread_hdls());
        Fullnode { exit, thread_hdls }
    }

    //used for notifying many nodes in parallel to exit
    pub fn exit(&self) {
        self.exit.store(true, Ordering::Relaxed);
    }
    pub fn close(self) -> Result<()> {
        self.exit();
        self.join()
    }
}

impl Service for Fullnode {
    fn thread_hdls(self) -> Vec<JoinHandle<()>> {
        self.thread_hdls
    }

    fn join(self) -> Result<()> {
        for thread_hdl in self.thread_hdls() {
            thread_hdl.join()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use bank::Bank;
    use crdt::TestNode;
    use fullnode::Fullnode;
    use mint::Mint;
    use service::Service;
    use signature::{Keypair, KeypairUtil};
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    #[test]
    fn validator_exit() {
        let keypair = Keypair::new();
        let tn = TestNode::new_localhost_with_pubkey(keypair.pubkey());
        let alice = Mint::new(10_000);
        let bank = Bank::new(&alice);
        let exit = Arc::new(AtomicBool::new(false));
        let entry = tn.data.clone();
        let v = Fullnode::new_validator(keypair, bank, 0, &[], tn, &entry, exit, None, false);
        v.exit();
        v.join().unwrap();
    }
    #[test]
    fn validator_parallel_exit() {
        let vals: Vec<Fullnode> = (0..2)
            .map(|_| {
                let keypair = Keypair::new();
                let tn = TestNode::new_localhost_with_pubkey(keypair.pubkey());
                let alice = Mint::new(10_000);
                let bank = Bank::new(&alice);
                let exit = Arc::new(AtomicBool::new(false));
                let entry = tn.data.clone();
                Fullnode::new_validator(keypair, bank, 0, &[], tn, &entry, exit, None, false)
            })
            .collect();
        //each validator can exit in parallel to speed many sequential calls to `join`
        vals.iter().for_each(|v| v.exit());
        //while join is called sequentially, the above exit call notified all the
        //validators to exit from all their threads
        vals.into_iter().for_each(|v| {
            v.join().unwrap();
        });
    }
}
