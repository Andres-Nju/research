//! The `replay_stage` replays transactions broadcast by the leader.

use crate::{
    cluster_info::ClusterInfo,
    cluster_info_vote_listener::VoteTracker,
    commitment::{AggregateCommitmentService, BlockCommitmentCache, CommitmentAggregationData},
    consensus::{StakeLockout, Tower},
    poh_recorder::{PohRecorder, GRACE_TICKS_FACTOR, MAX_GRACE_SLOTS},
    result::Result,
    rewards_recorder_service::RewardsRecorderSender,
    rpc_subscriptions::RpcSubscriptions,
};
use solana_ledger::{
    bank_forks::BankForks,
    block_error::BlockError,
    blockstore::Blockstore,
    blockstore_processor::{
        self, BlockstoreProcessorError, ConfirmationProgress, ConfirmationTiming,
        TransactionStatusSender,
    },
    entry::VerifyRecyclers,
    leader_schedule_cache::LeaderScheduleCache,
    snapshot_package::SnapshotPackageSender,
};
use solana_measure::thread_mem_usage;
use solana_metrics::inc_new_counter_info;
use solana_runtime::bank::Bank;
use solana_sdk::{
    clock::Slot,
    hash::Hash,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    timing::{self, duration_as_ms},
    transaction::Transaction,
};
use solana_vote_program::vote_instruction;
use std::{
    collections::{HashMap, HashSet},
    result,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{channel, Receiver, RecvTimeoutError, Sender},
        Arc, Mutex, RwLock,
    },
    thread::{self, Builder, JoinHandle},
    time::{Duration, Instant},
};

pub const MAX_ENTRY_RECV_PER_ITER: usize = 512;
pub(crate) type ProgressMap = HashMap<Slot, ForkProgress>;

#[derive(PartialEq, Debug)]
pub(crate) enum HeaviestForkFailures {
    LockedOut(u64),
    FailedThreshold(u64),
    FailedSwitchThreshold(u64),
}

// Implement a destructor for the ReplayStage thread to signal it exited
// even on panics
struct Finalizer {
    exit_sender: Arc<AtomicBool>,
}

impl Finalizer {
    fn new(exit_sender: Arc<AtomicBool>) -> Self {
        Finalizer { exit_sender }
    }
}

// Implement a destructor for Finalizer.
impl Drop for Finalizer {
    fn drop(&mut self) {
        self.exit_sender.clone().store(true, Ordering::Relaxed);
    }
}

#[derive(Default)]
pub struct ReplayStageConfig {
    pub my_pubkey: Pubkey,
    pub vote_account: Pubkey,
    pub voting_keypair: Option<Arc<Keypair>>,
    pub exit: Arc<AtomicBool>,
    pub subscriptions: Arc<RpcSubscriptions>,
    pub leader_schedule_cache: Arc<LeaderScheduleCache>,
    pub slot_full_senders: Vec<Sender<(u64, Pubkey)>>,
    pub latest_root_senders: Vec<Sender<Slot>>,
    pub snapshot_package_sender: Option<SnapshotPackageSender>,
    pub block_commitment_cache: Arc<RwLock<BlockCommitmentCache>>,
    pub transaction_status_sender: Option<TransactionStatusSender>,
    pub rewards_recorder_sender: Option<RewardsRecorderSender>,
}

pub struct ReplayStage {
    t_replay: JoinHandle<Result<()>>,
    commitment_service: AggregateCommitmentService,
}

#[derive(Default)]
pub(crate) struct ReplaySlotStats(ConfirmationTiming);
impl std::ops::Deref for ReplaySlotStats {
    type Target = ConfirmationTiming;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::ops::DerefMut for ReplaySlotStats {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ForkStats {
    weight: u128,
    fork_weight: u128,
    total_staked: u64,
    slot: Slot,
    block_height: u64,
    has_voted: bool,
    is_recent: bool,
    is_empty: bool,
    pub(crate) vote_threshold: bool,
    pub(crate) is_locked_out: bool,
    pub(crate) stake_lockouts: HashMap<u64, StakeLockout>,
    computed: bool,
    confirmation_reported: bool,
}

impl ReplaySlotStats {
    pub fn report_stats(&self, slot: Slot, num_entries: usize, num_shreds: u64) {
        datapoint_info!(
            "replay-slot-stats",
            ("slot", slot as i64, i64),
            ("fetch_entries_time", self.fetch_elapsed as i64, i64),
            (
                "fetch_entries_fail_time",
                self.fetch_fail_elapsed as i64,
                i64
            ),
            ("entry_verification_time", self.verify_elapsed as i64, i64),
            ("replay_time", self.replay_elapsed as i64, i64),
            (
                "replay_total_elapsed",
                self.started.elapsed().as_micros() as i64,
                i64
            ),
            ("total_entries", num_entries as i64, i64),
            ("total_shreds", num_shreds as i64, i64),
        );
    }
}

pub(crate) struct ForkProgress {
    is_dead: bool,
    pub(crate) fork_stats: ForkStats,
    replay_stats: ReplaySlotStats,
    replay_progress: ConfirmationProgress,
}

impl ForkProgress {
    pub fn new(last_entry: Hash) -> Self {
        Self {
            is_dead: false,
            fork_stats: ForkStats::default(),
            replay_stats: ReplaySlotStats::default(),
            replay_progress: ConfirmationProgress::new(last_entry),
        }
    }
}

impl ReplayStage {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(
        config: ReplayStageConfig,
        blockstore: Arc<Blockstore>,
        bank_forks: Arc<RwLock<BankForks>>,
        cluster_info: Arc<RwLock<ClusterInfo>>,
        ledger_signal_receiver: Receiver<bool>,
        poh_recorder: Arc<Mutex<PohRecorder>>,
        _vote_tracker: Arc<VoteTracker>,
    ) -> (Self, Receiver<Vec<Arc<Bank>>>) {
        let ReplayStageConfig {
            my_pubkey,
            vote_account,
            voting_keypair,
            exit,
            subscriptions,
            leader_schedule_cache,
            slot_full_senders,
            latest_root_senders,
            snapshot_package_sender,
            block_commitment_cache,
            transaction_status_sender,
            rewards_recorder_sender,
        } = config;

        let (root_bank_sender, root_bank_receiver) = channel();
        trace!("replay stage");
        let mut tower = Tower::new(&my_pubkey, &vote_account, &bank_forks.read().unwrap());

        // Start the replay stage loop

        let (lockouts_sender, commitment_service) =
            AggregateCommitmentService::new(&exit, block_commitment_cache);

        #[allow(clippy::cognitive_complexity)]
        let t_replay = Builder::new()
            .name("solana-replay-stage".to_string())
            .spawn(move || {
                let verify_recyclers = VerifyRecyclers::default();
                let _exit = Finalizer::new(exit.clone());
                let mut progress = HashMap::new();
                // Initialize progress map with any root banks
                for bank in bank_forks.read().unwrap().frozen_banks().values() {
                    progress.insert(bank.slot(), ForkProgress::new(bank.last_blockhash()));
                }
                let mut current_leader = None;
                let mut last_reset = Hash::default();
                let mut partition = false;
                let mut earliest_vote_on_fork = {
                    let slots = tower.last_vote().slots;
                    slots.last().cloned().unwrap_or(0)
                };
                let mut switch_threshold = false;
                loop {
                    let allocated = thread_mem_usage::Allocatedp::default();

                    thread_mem_usage::datapoint("solana-replay-stage");
                    let now = Instant::now();
                    // Stop getting entries if we get exit signal
                    if exit.load(Ordering::Relaxed) {
                        break;
                    }

                    let start = allocated.get();
                    Self::generate_new_bank_forks(
                        &blockstore,
                        &bank_forks,
                        &leader_schedule_cache,
                        &subscriptions,
                        rewards_recorder_sender.clone(),
                    );
                    Self::report_memory(&allocated, "generate_new_bank_forks", start);

                    let mut tpu_has_bank = poh_recorder.lock().unwrap().has_bank();

                    let start = allocated.get();
                    let did_complete_bank = Self::replay_active_banks(
                        &blockstore,
                        &bank_forks,
                        &my_pubkey,
                        &mut progress,
                        &slot_full_senders,
                        transaction_status_sender.clone(),
                        &verify_recyclers,
                    );
                    Self::report_memory(&allocated, "replay_active_banks", start);

                    let ancestors = Arc::new(bank_forks.read().unwrap().ancestors());
                    let descendants = Arc::new(HashMap::new());
                    let start = allocated.get();
                    let mut frozen_banks: Vec<_> = bank_forks
                        .read()
                        .unwrap()
                        .frozen_banks()
                        .values()
                        .cloned()
                        .collect();
                    let newly_computed_slot_stats = Self::compute_bank_stats(
                        &my_pubkey,
                        &ancestors,
                        &mut frozen_banks,
                        &tower,
                        &mut progress,
                    );
                    for slot in newly_computed_slot_stats {
                        let fork_stats = &progress.get(&slot).unwrap().fork_stats;
                        let confirmed_forks = Self::confirm_forks(
                            &tower,
                            &fork_stats.stake_lockouts,
                            fork_stats.total_staked,
                            &progress,
                            &bank_forks,
                        );

                        for slot in confirmed_forks {
                            progress
                                .get_mut(&slot)
                                .unwrap()
                                .fork_stats
                                .confirmation_reported = true;
                        }
                    }

                    let (heaviest_bank, votable_bank_on_same_fork) =
                        Self::select_forks(&frozen_banks, &tower, &progress, &ancestors);

                    Self::report_memory(&allocated, "select_fork", start);

                    let (vote_bank, reset_bank, failure_reasons) =
                        Self::select_vote_and_reset_forks(
                            &heaviest_bank,
                            &votable_bank_on_same_fork,
                            earliest_vote_on_fork,
                            &mut switch_threshold,
                            &ancestors,
                            &descendants,
                            &progress,
                            &tower,
                        );

                    if heaviest_bank.is_some()
                        && tower.is_recent(heaviest_bank.as_ref().unwrap().slot())
                        && !failure_reasons.is_empty()
                    {
                        info!(
                            "Couldn't vote on heaviest fork: {:?}, failure_reasons: {:?}",
                            heaviest_bank.as_ref().map(|b| b.slot()),
                            failure_reasons
                        );
                    }

                    let start = allocated.get();

                    // Vote on a fork
                    let voted_on_different_fork = {
                        if let Some(ref vote_bank) = vote_bank {
                            subscriptions.notify_subscribers(vote_bank.slot(), &bank_forks);
                            if let Some(votable_leader) = leader_schedule_cache
                                .slot_leader_at(vote_bank.slot(), Some(vote_bank))
                            {
                                Self::log_leader_change(
                                    &my_pubkey,
                                    vote_bank.slot(),
                                    &mut current_leader,
                                    &votable_leader,
                                );
                            }

                            Self::handle_votable_bank(
                                &vote_bank,
                                &bank_forks,
                                &mut tower,
                                &mut progress,
                                &vote_account,
                                &voting_keypair,
                                &cluster_info,
                                &blockstore,
                                &leader_schedule_cache,
                                &root_bank_sender,
                                &lockouts_sender,
                                &snapshot_package_sender,
                                &latest_root_senders,
                                &mut earliest_vote_on_fork,
                            )?;

                            ancestors
                                .get(&vote_bank.slot())
                                .unwrap()
                                .contains(&earliest_vote_on_fork)
                        } else {
                            false
                        }
                    };

                    Self::report_memory(&allocated, "votable_bank", start);
                    let start = allocated.get();

                    // Reset onto a fork
                    if let Some(reset_bank) = reset_bank {
                        let selected_same_fork = ancestors
                            .get(&reset_bank.slot())
                            .unwrap()
                            .contains(&earliest_vote_on_fork);
                        if last_reset != reset_bank.last_blockhash()
                            && (selected_same_fork || switch_threshold)
                        {
                            info!(
                                "vote bank: {:?} reset bank: {:?}",
                                vote_bank.as_ref().map(|b| b.slot()),
                                reset_bank.slot(),
                            );
                            Self::reset_poh_recorder(
                                &my_pubkey,
                                &blockstore,
                                &reset_bank,
                                &poh_recorder,
                                &leader_schedule_cache,
                            );
                            last_reset = reset_bank.last_blockhash();
                            tpu_has_bank = false;

                            if !partition
                                && vote_bank.as_ref().map(|b| b.slot()) != Some(reset_bank.slot())
                            {
                                warn!(
                                    "PARTITION DETECTED waiting to join fork: {} last vote: {:?}",
                                    reset_bank.slot(),
                                    tower.last_vote()
                                );
                                inc_new_counter_info!("replay_stage-partition_detected", 1);
                                datapoint_info!(
                                    "replay_stage-partition",
                                    ("slot", reset_bank.slot() as i64, i64)
                                );
                                partition = true;
                            } else if partition
                                && vote_bank.as_ref().map(|b| b.slot()) == Some(reset_bank.slot())
                            {
                                warn!(
                                    "PARTITION resolved fork: {} last vote: {:?}",
                                    reset_bank.slot(),
                                    tower.last_vote()
                                );
                                partition = false;
                                inc_new_counter_info!("replay_stage-partition_resolved", 1);
                            }
                        }
                        datapoint_debug!(
                            "replay_stage-memory",
                            ("reset_bank", (allocated.get() - start) as i64, i64),
                        );
                    }
                    Self::report_memory(&allocated, "reset_bank", start);

                    // If we voted on a different fork, update the earliest vote
                    // to this slot, clear the switch threshold
                    if voted_on_different_fork {
                        earliest_vote_on_fork = vote_bank
                            .expect("voted_on_different_fork only set if vote_bank.is_some()")
                            .slot();
                        // Clear the thresholds after voting on different
                        // fork
                        switch_threshold = false;
                    }

                    let start = allocated.get();
                    if !tpu_has_bank {
                        Self::maybe_start_leader(
                            &my_pubkey,
                            &bank_forks,
                            &poh_recorder,
                            &leader_schedule_cache,
                            &subscriptions,
                            rewards_recorder_sender.clone(),
                        );

                        let poh_bank = poh_recorder.lock().unwrap().bank();
                        if let Some(bank) = poh_bank {
                            Self::log_leader_change(
                                &my_pubkey,
                                bank.slot(),
                                &mut current_leader,
                                &my_pubkey,
                            );
                        }
                    }
                    Self::report_memory(&allocated, "start_leader", start);
                    datapoint_debug!(
                        "replay_stage",
                        ("duration", duration_as_ms(&now.elapsed()) as i64, i64)
                    );
                    if did_complete_bank {
                        //just processed a bank, skip the signal; maybe there's more slots available
                        continue;
                    }
                    let timer = Duration::from_millis(100);
                    let result = ledger_signal_receiver.recv_timeout(timer);
                    match result {
                        Err(RecvTimeoutError::Timeout) => continue,
                        Err(_) => break,
                        Ok(_) => trace!("blockstore signal"),
                    };
                }
                Ok(())
            })
            .unwrap();
        (
            Self {
                t_replay,
                commitment_service,
            },
            root_bank_receiver,
        )
    }

    fn report_memory(
        allocated: &solana_measure::thread_mem_usage::Allocatedp,
        name: &'static str,
        start: u64,
    ) {
        datapoint_debug!(
            "replay_stage-memory",
            (name, (allocated.get() - start) as i64, i64),
        );
    }

    fn log_leader_change(
        my_pubkey: &Pubkey,
        bank_slot: Slot,
        current_leader: &mut Option<Pubkey>,
        new_leader: &Pubkey,
    ) {
        if let Some(ref current_leader) = current_leader {
            if current_leader != new_leader {
                let msg = if current_leader == my_pubkey {
                    ". I am no longer the leader"
                } else if new_leader == my_pubkey {
                    ". I am now the leader"
                } else {
                    ""
                };
                info!(
                    "LEADER CHANGE at slot: {} leader: {}{}",
                    bank_slot, new_leader, msg
                );
            }
        }
        current_leader.replace(new_leader.to_owned());
    }

    fn maybe_start_leader(
        my_pubkey: &Pubkey,
        bank_forks: &Arc<RwLock<BankForks>>,
        poh_recorder: &Arc<Mutex<PohRecorder>>,
        leader_schedule_cache: &Arc<LeaderScheduleCache>,
        subscriptions: &Arc<RpcSubscriptions>,
        rewards_recorder_sender: Option<RewardsRecorderSender>,
    ) {
        // all the individual calls to poh_recorder.lock() are designed to
        // increase granularity, decrease contention

        assert!(!poh_recorder.lock().unwrap().has_bank());

        let (reached_leader_slot, _grace_ticks, poh_slot, parent_slot) =
            poh_recorder.lock().unwrap().reached_leader_slot();

        if !reached_leader_slot {
            trace!("{} poh_recorder hasn't reached_leader_slot", my_pubkey);
            return;
        }
        trace!("{} reached_leader_slot", my_pubkey);

        let parent = bank_forks
            .read()
            .unwrap()
            .get(parent_slot)
            .expect("parent_slot doesn't exist in bank forks")
            .clone();

        assert!(parent.is_frozen());

        if bank_forks.read().unwrap().get(poh_slot).is_some() {
            warn!("{} already have bank in forks at {}?", my_pubkey, poh_slot);
            return;
        }
        trace!(
            "{} poh_slot {} parent_slot {}",
            my_pubkey,
            poh_slot,
            parent_slot
        );

        if let Some(next_leader) = leader_schedule_cache.slot_leader_at(poh_slot, Some(&parent)) {
            trace!(
                "{} leader {} at poh slot: {}",
                my_pubkey,
                next_leader,
                poh_slot
            );

            // I guess I missed my slot
            if next_leader != *my_pubkey {
                return;
            }

            datapoint_info!(
                "replay_stage-new_leader",
                ("slot", poh_slot, i64),
                ("leader", next_leader.to_string(), String),
            );

            let root_slot = bank_forks.read().unwrap().root();
            info!(
                "new fork:{} parent:{} (leader) root:{}",
                poh_slot, parent_slot, root_slot
            );

            let tpu_bank = Self::new_bank_from_parent_with_notify(
                &parent,
                poh_slot,
                root_slot,
                my_pubkey,
                &rewards_recorder_sender,
                subscriptions,
            );

            let tpu_bank = bank_forks.write().unwrap().insert(tpu_bank);
            poh_recorder.lock().unwrap().set_bank(&tpu_bank);
        } else {
            error!("{} No next leader found", my_pubkey);
        }
    }

    fn replay_blockstore_into_bank(
        bank: &Arc<Bank>,
        blockstore: &Blockstore,
        bank_progress: &mut ForkProgress,
        transaction_status_sender: Option<TransactionStatusSender>,
        verify_recyclers: &VerifyRecyclers,
    ) -> result::Result<usize, BlockstoreProcessorError> {
        let tx_count_before = bank_progress.replay_progress.num_txs;
        let confirm_result = blockstore_processor::confirm_slot(
            blockstore,
            bank,
            &mut bank_progress.replay_stats,
            &mut bank_progress.replay_progress,
            false,
            transaction_status_sender,
            None,
            verify_recyclers,
        );
        let tx_count_after = bank_progress.replay_progress.num_txs;
        let tx_count = tx_count_after - tx_count_before;

        confirm_result.map_err(|err| {
            // LedgerCleanupService should not be cleaning up anything
            // that comes after the root, so we should not see any
            // errors related to the slot being purged
            let slot = bank.slot();
            warn!("Fatal replay error in slot: {}, err: {:?}", slot, err);
            if let BlockstoreProcessorError::InvalidBlock(BlockError::InvalidTickCount) = err {
                datapoint_info!(
                    "replay-stage-mark_dead_slot",
                    ("error", format!("error: {:?}", err), String),
                    ("slot", slot, i64)
                );
            } else {
                datapoint_error!(
                    "replay-stage-mark_dead_slot",
                    ("error", format!("error: {:?}", err), String),
                    ("slot", slot, i64)
                );
            }
            bank_progress.is_dead = true;
            blockstore
                .set_dead_slot(slot)
                .expect("Failed to mark slot as dead in blockstore");
            err
        })?;

        Ok(tx_count)
    }

    #[allow(clippy::too_many_arguments)]
    fn handle_votable_bank(
        bank: &Arc<Bank>,
        bank_forks: &Arc<RwLock<BankForks>>,
        tower: &mut Tower,
        progress: &mut ProgressMap,
        vote_account: &Pubkey,
        voting_keypair: &Option<Arc<Keypair>>,
        cluster_info: &Arc<RwLock<ClusterInfo>>,
        blockstore: &Arc<Blockstore>,
        leader_schedule_cache: &Arc<LeaderScheduleCache>,
        root_bank_sender: &Sender<Vec<Arc<Bank>>>,
        lockouts_sender: &Sender<CommitmentAggregationData>,
        snapshot_package_sender: &Option<SnapshotPackageSender>,
        latest_root_senders: &[Sender<Slot>],
        earliest_vote_on_fork: &mut Slot,
    ) -> Result<()> {
        if bank.is_empty() {
            inc_new_counter_info!("replay_stage-voted_empty_bank", 1);
        }
        trace!("handle votable bank {}", bank.slot());
        let (vote, tower_index) = tower.new_vote_from_bank(bank, vote_account);
        if let Some(new_root) = tower.record_bank_vote(vote) {
            // get the root bank before squash
            let root_bank = bank_forks
                .read()
                .unwrap()
                .get(new_root)
                .expect("Root bank doesn't exist")
                .clone();
            let mut rooted_banks = root_bank.parents();
            rooted_banks.push(root_bank);
            let rooted_slots: Vec<_> = rooted_banks.iter().map(|bank| bank.slot()).collect();
            // Call leader schedule_cache.set_root() before blockstore.set_root() because
            // bank_forks.root is consumed by repair_service to update gossip, so we don't want to
            // get shreds for repair on gossip before we update leader schedule, otherwise they may
            // get dropped.
            leader_schedule_cache.set_root(rooted_banks.last().unwrap());
            blockstore
                .set_roots(&rooted_slots)
                .expect("Ledger set roots failed");
            Self::handle_new_root(
                new_root,
                &bank_forks,
                progress,
                snapshot_package_sender,
                earliest_vote_on_fork,
            );
            latest_root_senders.iter().for_each(|s| {
                if let Err(e) = s.send(new_root) {
                    trace!("latest root send failed: {:?}", e);
                }
            });
            trace!("new root {}", new_root);
            if let Err(e) = root_bank_sender.send(rooted_banks) {
                trace!("root_bank_sender failed: {:?}", e);
                return Err(e.into());
            }
        }

        Self::update_commitment_cache(
            bank.clone(),
            progress.get(&bank.slot()).unwrap().fork_stats.total_staked,
            lockouts_sender,
        );

        if let Some(ref voting_keypair) = voting_keypair {
            let node_keypair = cluster_info.read().unwrap().keypair.clone();

            // Send our last few votes along with the new one
            let vote_ix = vote_instruction::vote(
                &vote_account,
                &voting_keypair.pubkey(),
                tower.last_vote_and_timestamp(),
            );

            let mut vote_tx =
                Transaction::new_with_payer(vec![vote_ix], Some(&node_keypair.pubkey()));

            let blockhash = bank.last_blockhash();
            vote_tx.partial_sign(&[node_keypair.as_ref()], blockhash);
            vote_tx.partial_sign(&[voting_keypair.as_ref()], blockhash);
            cluster_info
                .write()
                .unwrap()
                .push_vote(tower_index, vote_tx);
        }
        Ok(())
    }

    fn update_commitment_cache(
        bank: Arc<Bank>,
        total_staked: u64,
        lockouts_sender: &Sender<CommitmentAggregationData>,
    ) {
        if let Err(e) = lockouts_sender.send(CommitmentAggregationData::new(bank, total_staked)) {
            trace!("lockouts_sender failed: {:?}", e);
        }
    }

    fn reset_poh_recorder(
        my_pubkey: &Pubkey,
        blockstore: &Blockstore,
        bank: &Arc<Bank>,
        poh_recorder: &Arc<Mutex<PohRecorder>>,
        leader_schedule_cache: &Arc<LeaderScheduleCache>,
    ) {
        let next_leader_slot = leader_schedule_cache.next_leader_slot(
            &my_pubkey,
            bank.slot(),
            &bank,
            Some(blockstore),
            GRACE_TICKS_FACTOR * MAX_GRACE_SLOTS,
        );
        poh_recorder
            .lock()
            .unwrap()
            .reset(bank.last_blockhash(), bank.slot(), next_leader_slot);

        let next_leader_msg = if let Some(next_leader_slot) = next_leader_slot {
            format!("My next leader slot is {}", next_leader_slot.0)
        } else {
            "I am not in the leader schedule yet".to_owned()
        };

        info!(
            "{} reset PoH to tick {} (within slot {}). {}",
            my_pubkey,
            bank.tick_height(),
            bank.slot(),
            next_leader_msg,
        );
    }

    fn replay_active_banks(
        blockstore: &Arc<Blockstore>,
        bank_forks: &Arc<RwLock<BankForks>>,
        my_pubkey: &Pubkey,
        progress: &mut ProgressMap,
        slot_full_senders: &[Sender<(u64, Pubkey)>],
        transaction_status_sender: Option<TransactionStatusSender>,
        verify_recyclers: &VerifyRecyclers,
    ) -> bool {
        let mut did_complete_bank = false;
        let mut tx_count = 0;
        let active_banks = bank_forks.read().unwrap().active_banks();
        trace!("active banks {:?}", active_banks);

        for bank_slot in &active_banks {
            // If the fork was marked as dead, don't replay it
            if progress.get(bank_slot).map(|p| p.is_dead).unwrap_or(false) {
                debug!("bank_slot {:?} is marked dead", *bank_slot);
                continue;
            }

            let bank = bank_forks.read().unwrap().get(*bank_slot).unwrap().clone();

            // Insert a progress entry even for slots this node is the leader for, so that
            // 1) confirm_forks can report confirmation, 2) we can cache computations about
            // this bank in `select_forks()`
            let bank_progress = &mut progress
                .entry(bank.slot())
                .or_insert_with(|| ForkProgress::new(bank.last_blockhash()));
            if bank.collector_id() != my_pubkey {
                let replay_result = Self::replay_blockstore_into_bank(
                    &bank,
                    &blockstore,
                    bank_progress,
                    transaction_status_sender.clone(),
                    verify_recyclers,
                );
                match replay_result {
                    Ok(replay_tx_count) => tx_count += replay_tx_count,
                    Err(err) => {
                        trace!("replay_result err: {:?}, slot {}", err, bank_slot);
                        // If the bank was corrupted, don't try to run the below logic to check if the
                        // bank is completed
                        continue;
                    }
                }
            }
            assert_eq!(*bank_slot, bank.slot());
            if bank.is_complete() {
                bank_progress.replay_stats.report_stats(
                    bank.slot(),
                    bank_progress.replay_progress.num_entries,
                    bank_progress.replay_progress.num_shreds,
                );
                did_complete_bank = true;
                Self::process_completed_bank(my_pubkey, bank, slot_full_senders);
            } else {
                trace!(
                    "bank {} not completed tick_height: {}, max_tick_height: {}",
                    bank.slot(),
                    bank.tick_height(),
                    bank.max_tick_height()
                );
            }
        }
        inc_new_counter_info!("replay_stage-replay_transactions", tx_count);
        did_complete_bank
    }

    pub(crate) fn compute_bank_stats(
        my_pubkey: &Pubkey,
        ancestors: &HashMap<u64, HashSet<u64>>,
        frozen_banks: &mut Vec<Arc<Bank>>,
        tower: &Tower,
        progress: &mut ProgressMap,
    ) -> Vec<Slot> {
        frozen_banks.sort_by_key(|bank| bank.slot());
        let mut new_stats = vec![];
        for bank in frozen_banks {
            // Only time progress map should be missing a bank slot
            // is if this node was the leader for this slot as those banks
            // are not replayed in replay_active_banks()
            let parent_weight = bank
                .parent()
                .and_then(|b| progress.get(&b.slot()))
                .map(|x| x.fork_stats.fork_weight)
                .unwrap_or(0);
            let stats = &mut progress
                .get_mut(&bank.slot())
                .expect("All frozen banks must exist in the Progress map")
                .fork_stats;

            if !stats.computed {
                stats.slot = bank.slot();
                let (stake_lockouts, total_staked, bank_weight) = tower.collect_vote_lockouts(
                    bank.slot(),
                    bank.vote_accounts().into_iter(),
                    &ancestors,
                );
                stats.total_staked = total_staked;
                stats.weight = bank_weight;
                stats.fork_weight = stats.weight + parent_weight;

                datapoint_info!(
                    "bank_weight",
                    ("slot", bank.slot(), i64),
                    // u128 too large for influx, convert to hex
                    ("weight", format!("{:X}", stats.weight), String),
                );
                info!(
                    "{} slot_weight: {} {} {} {}",
                    my_pubkey,
                    stats.slot,
                    stats.weight,
                    stats.fork_weight,
                    bank.parent().map(|b| b.slot()).unwrap_or(0)
                );
                stats.stake_lockouts = stake_lockouts;
                stats.block_height = bank.block_height();
                stats.computed = true;
                new_stats.push(stats.slot);
            }
            stats.vote_threshold = tower.check_vote_stake_threshold(
                bank.slot(),
                &stats.stake_lockouts,
                stats.total_staked,
            );
            stats.is_locked_out = tower.is_locked_out(bank.slot(), &ancestors);
            stats.has_voted = tower.has_voted(bank.slot());
            stats.is_recent = tower.is_recent(bank.slot());
        }
        new_stats
    }

    // Returns:
    // 1) The heaviest bank
    // 2) The latest votable bank on the same fork as the last vote
    pub(crate) fn select_forks(
        frozen_banks: &[Arc<Bank>],
        tower: &Tower,
        progress: &ProgressMap,
        ancestors: &HashMap<u64, HashSet<u64>>,
    ) -> (Option<Arc<Bank>>, Option<Arc<Bank>>) {
        let tower_start = Instant::now();
        let num_frozen_banks = frozen_banks.len();

        trace!("frozen_banks {}", frozen_banks.len());
        let num_old_banks = frozen_banks
            .iter()
            .filter(|b| b.slot() < tower.root().unwrap_or(0))
            .count();

        let last_vote = tower.last_vote().slots.last().cloned();
        let mut last_votable_on_same_fork = None;
        let stats: Vec<&ForkStats> = frozen_banks
            .iter()
            .map(|bank| {
                // Only time progress map should be missing a bank slot
                // is if this node was the leader for this slot as those banks
                // are not replayed in replay_active_banks()
                let stats = &progress
                    .get(&bank.slot())
                    .expect("All frozen banks must exist in the Progress map")
                    .fork_stats;

                if let Some(last_vote) = last_vote {
                    if ancestors
                        .get(&bank.slot())
                        .expect("Entry in frozen banks must exist in ancestors")
                        .contains(&last_vote)
                        && stats.vote_threshold
                    {
                        // Descendant of last vote cannot be locked out
                        assert!(!stats.is_locked_out);

                        // ancestors(slot) should not contain the slot itself,
                        // so we shouldd never get the same bank as the last vote
                        assert_ne!(bank.slot(), last_vote);
                        last_votable_on_same_fork = Some(bank.clone());
                    }
                }

                stats
            })
            .collect();
        let num_not_recent = stats.iter().filter(|s| !s.is_recent).count();
        let num_has_voted = stats.iter().filter(|s| s.has_voted).count();
        let num_empty = stats.iter().filter(|s| s.is_empty).count();
        let num_threshold_failure = stats.iter().filter(|s| !s.vote_threshold).count();
        let num_votable_threshold_failure = stats
            .iter()
            .filter(|s| s.is_recent && !s.has_voted && !s.vote_threshold)
            .count();

        let mut candidates: Vec<_> = frozen_banks.iter().zip(stats.iter()).collect();

        //highest weight, lowest slot first
        candidates.sort_by_key(|b| (b.1.fork_weight, 0i64 - b.1.slot as i64));
        let rv = candidates.last();
        let ms = timing::duration_as_ms(&tower_start.elapsed());
        let weights: Vec<(u128, u64, u64)> = candidates
            .iter()
            .map(|x| (x.1.weight, x.1.slot, x.1.block_height))
            .collect();
        debug!(
            "@{:?} tower duration: {:?} len: {}/{} weights: {:?} voting: {}",
            timing::timestamp(),
            ms,
            candidates.len(),
            stats.iter().filter(|s| !s.has_voted).count(),
            weights,
            rv.is_some()
        );
        datapoint_debug!(
            "replay_stage-select_forks",
            ("frozen_banks", num_frozen_banks as i64, i64),
            ("not_recent", num_not_recent as i64, i64),
            ("has_voted", num_has_voted as i64, i64),
            ("old_banks", num_old_banks as i64, i64),
            ("empty_banks", num_empty as i64, i64),
            ("threshold_failure", num_threshold_failure as i64, i64),
            (
                "votable_threshold_failure",
                num_votable_threshold_failure as i64,
                i64
            ),
            ("tower_duration", ms as i64, i64),
        );

        (rv.map(|x| x.0.clone()), last_votable_on_same_fork)
    }

    // Given a heaviest bank, `heaviest_bank` and the next votable bank
    // `votable_bank_on_same_fork` as the validator's last vote, return
    // a bank to vote on, a bank to reset to,
    pub(crate) fn select_vote_and_reset_forks(
        heaviest_bank: &Option<Arc<Bank>>,
        votable_bank_on_same_fork: &Option<Arc<Bank>>,
        earliest_vote_on_fork: u64,
        switch_threshold: &mut bool,
        ancestors: &HashMap<u64, HashSet<u64>>,
        descendants: &HashMap<u64, HashSet<u64>>,
        progress: &ProgressMap,
        tower: &Tower,
    ) -> (
        Option<Arc<Bank>>,
        Option<Arc<Bank>>,
        Vec<HeaviestForkFailures>,
    ) {
        // Try to vote on the actual heaviest fork. If the heaviest bank is
        // locked out or fails the threshold check, the validator will:
        // 1) Not continue to vote on current fork, waiting for lockouts to expire/
        //    threshold check to pass
        // 2) Will reset PoH to heaviest fork in order to make sure the heaviest
        //    fork is propagated
        // This above behavior should ensure correct voting and resetting PoH
        // behavior under all cases:
        // 1) The best "selected" bank is on same fork
        // 2) The best "selected" bank is on a different fork,
        //    switch_threshold fails
        // 3) The best "selected" bank is on a different fork,
        //    switch_threshold succceeds
        let mut failure_reasons = vec![];
        let selected_fork = {
            if let Some(bank) = heaviest_bank {
                let selected_same_fork = ancestors
                    .get(&bank.slot())
                    .unwrap()
                    .contains(&earliest_vote_on_fork);
                if selected_same_fork {
                    // If the heaviest bank is on the same fork as the last
                    // vote, then there's no need to check the switch threshold.
                    // Just vote for the latest votable bank on the same fork,
                    // which is `votable_bank_on_same_fork`.
                    votable_bank_on_same_fork
                } else {
                    if !*switch_threshold {
                        let total_staked =
                            progress.get(&bank.slot()).unwrap().fork_stats.total_staked;
                        *switch_threshold = tower.check_switch_threshold(
                            earliest_vote_on_fork,
                            &ancestors,
                            &descendants,
                            &progress,
                            total_staked,
                        );
                    }
                    if !*switch_threshold {
                        // If we can't switch, then vote on the the next votable
                        // bank on the same fork as our last vote
                        info!(
                            "Waiting to switch to {}, voting on {:?} on same fork for now",
                            bank.slot(),
                            votable_bank_on_same_fork.as_ref().map(|b| b.slot())
                        );
                        failure_reasons
                            .push(HeaviestForkFailures::FailedSwitchThreshold(bank.slot()));
                        votable_bank_on_same_fork
                    } else {
                        // If the switch threshold is observed, halt voting on
                        // the current fork and attempt to vote/reset Poh/switch to
                        // theh heaviest bank
                        heaviest_bank
                    }
                }
            } else {
                &None
            }
        };

        if let Some(bank) = selected_fork {
            let (is_locked_out, vote_threshold, fork_weight) = {
                let fork_stats = &progress.get(&bank.slot()).unwrap().fork_stats;
                (
                    fork_stats.is_locked_out,
                    fork_stats.vote_threshold,
                    fork_stats.weight,
                )
            };
            if is_locked_out {
                failure_reasons.push(HeaviestForkFailures::LockedOut(bank.slot()));
            }
            if !vote_threshold {
                failure_reasons.push(HeaviestForkFailures::FailedThreshold(bank.slot()));
            }

            if !is_locked_out && vote_threshold {
                info!("voting: {} {}", bank.slot(), fork_weight);
                (
                    selected_fork.clone(),
                    selected_fork.clone(),
                    failure_reasons,
                )
            } else {
                (None, selected_fork.clone(), failure_reasons)
            }
        } else {
            (None, None, failure_reasons)
        }
    }

    fn confirm_forks(
        tower: &Tower,
        stake_lockouts: &HashMap<u64, StakeLockout>,
        total_staked: u64,
        progress: &ProgressMap,
        bank_forks: &RwLock<BankForks>,
    ) -> Vec<Slot> {
        let mut confirmed_forks = vec![];
        for (slot, prog) in progress.iter() {
            if !prog.fork_stats.confirmation_reported {
                let bank = bank_forks
                    .read()
                    .unwrap()
                    .get(*slot)
                    .expect("bank in progress must exist in BankForks")
                    .clone();
                let duration = prog.replay_stats.started.elapsed().as_millis();
                if bank.is_frozen() && tower.is_slot_confirmed(*slot, stake_lockouts, total_staked)
                {
                    info!("validator fork confirmed {} {}ms", *slot, duration);
                    datapoint_info!("validator-confirmation", ("duration_ms", duration, i64));
                    confirmed_forks.push(*slot);
                } else {
                    debug!(
                        "validator fork not confirmed {} {}ms {:?}",
                        *slot,
                        duration,
                        stake_lockouts.get(slot)
                    );
                }
            }
        }
        confirmed_forks
    }

    pub(crate) fn handle_new_root(
        new_root: u64,
        bank_forks: &RwLock<BankForks>,
        progress: &mut ProgressMap,
        snapshot_package_sender: &Option<SnapshotPackageSender>,
        earliest_vote_on_fork: &mut u64,
    ) {
        bank_forks
            .write()
            .unwrap()
            .set_root(new_root, snapshot_package_sender);
        let r_bank_forks = bank_forks.read().unwrap();
        *earliest_vote_on_fork = std::cmp::max(new_root, *earliest_vote_on_fork);
        progress.retain(|k, _| r_bank_forks.get(*k).is_some());
    }

    fn process_completed_bank(
        my_pubkey: &Pubkey,
        bank: Arc<Bank>,
        slot_full_senders: &[Sender<(u64, Pubkey)>],
    ) {
        info!("bank frozen: {}", bank.slot());
        bank.freeze();
        slot_full_senders.iter().for_each(|sender| {
            if let Err(e) = sender.send((bank.slot(), *bank.collector_id())) {
                trace!("{} slot_full alert failed: {:?}", my_pubkey, e);
            }
        });
    }

    fn generate_new_bank_forks(
        blockstore: &Blockstore,
        forks_lock: &RwLock<BankForks>,
        leader_schedule_cache: &Arc<LeaderScheduleCache>,
        subscriptions: &Arc<RpcSubscriptions>,
        rewards_recorder_sender: Option<RewardsRecorderSender>,
    ) {
        // Find the next slot that chains to the old slot
        let forks = forks_lock.read().unwrap();
        let frozen_banks = forks.frozen_banks();
        let frozen_bank_slots: Vec<u64> = frozen_banks.keys().cloned().collect();
        let next_slots = blockstore
            .get_slots_since(&frozen_bank_slots)
            .expect("Db error");
        // Filter out what we've already seen
        trace!("generate new forks {:?}", {
            let mut next_slots = next_slots.iter().collect::<Vec<_>>();
            next_slots.sort();
            next_slots
        });
        let mut new_banks = HashMap::new();
        for (parent_slot, children) in next_slots {
            let parent_bank = frozen_banks
                .get(&parent_slot)
                .expect("missing parent in bank forks")
                .clone();
            for child_slot in children {
                if forks.get(child_slot).is_some() || new_banks.get(&child_slot).is_some() {
                    trace!("child already active or frozen {}", child_slot);
                    continue;
                }
                let leader = leader_schedule_cache
                    .slot_leader_at(child_slot, Some(&parent_bank))
                    .unwrap();
                info!(
                    "new fork:{} parent:{} root:{}",
                    child_slot,
                    parent_slot,
                    forks.root()
                );
                let child_bank = Self::new_bank_from_parent_with_notify(
                    &parent_bank,
                    child_slot,
                    forks.root(),
                    &leader,
                    &rewards_recorder_sender,
                    subscriptions,
                );
                new_banks.insert(child_slot, child_bank);
            }
        }
        drop(forks);

        let mut forks = forks_lock.write().unwrap();
        for (_, bank) in new_banks {
            forks.insert(bank);
        }
    }

    fn new_bank_from_parent_with_notify(
        parent: &Arc<Bank>,
        slot: u64,
        root_slot: u64,
        leader: &Pubkey,
        rewards_recorder_sender: &Option<RewardsRecorderSender>,
        subscriptions: &Arc<RpcSubscriptions>,
    ) -> Bank {
        subscriptions.notify_slot(slot, parent.slot(), root_slot);

        let child_bank = Bank::new_from_parent(parent, leader, slot);
        Self::record_rewards(&child_bank, &rewards_recorder_sender);
        child_bank
    }

    fn record_rewards(bank: &Bank, rewards_recorder_sender: &Option<RewardsRecorderSender>) {
        if let Some(rewards_recorder_sender) = rewards_recorder_sender {
            if let Some(ref rewards) = bank.rewards {
                rewards_recorder_sender
                    .send((bank.slot(), rewards.iter().copied().collect()))
                    .unwrap_or_else(|err| warn!("rewards_recorder_sender failed: {:?}", err));
            }
        }
    }

    pub fn join(self) -> thread::Result<()> {
        self.commitment_service.join()?;
        self.t_replay.join().map(|_| ())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::{
        commitment::BlockCommitment,
        consensus::test::{initialize_state, VoteSimulator},
        consensus::Tower,
        genesis_utils::{create_genesis_config, create_genesis_config_with_leader},
        replay_stage::ReplayStage,
        transaction_status_service::TransactionStatusService,
    };
    use crossbeam_channel::unbounded;
    use solana_client::rpc_response::{RpcEncodedTransaction, RpcTransactionWithStatusMeta};
    use solana_ledger::{
        blockstore::make_slot_entries,
        blockstore::{entries_to_test_shreds, BlockstoreError},
        create_new_tmp_ledger,
        entry::{self, next_entry, Entry},
        get_tmp_ledger_path,
        shred::{
            CodingShredHeader, DataShredHeader, Shred, ShredCommonHeader, DATA_COMPLETE_SHRED,
            SIZE_OF_COMMON_SHRED_HEADER, SIZE_OF_DATA_SHRED_HEADER, SIZE_OF_DATA_SHRED_PAYLOAD,
        },
    };
    use solana_runtime::genesis_utils::{GenesisConfigInfo, ValidatorVoteKeypairs};
    use solana_sdk::{
        account::Account,
        hash::{hash, Hash},
        instruction::InstructionError,
        packet::PACKET_DATA_SIZE,
        rent::Rent,
        signature::{Keypair, Signature, Signer},
        system_transaction,
        transaction::TransactionError,
    };
    use solana_stake_program::stake_state;
    use solana_vote_program::{
        vote_state::{self, Vote, VoteState, VoteStateVersions},
        vote_transaction,
    };
    use std::{
        fs::remove_dir_all,
        iter,
        sync::{Arc, RwLock},
    };
    use trees::tr;

    struct ForkInfo {
        leader: usize,
        fork: Vec<Slot>,
        voters: Vec<usize>,
    }

    struct ValidatorInfo {
        stake: u64,
        keypair: Keypair,
        voting_keypair: Keypair,
        staking_keypair: Keypair,
    }

    struct ForkSelectionResponse {
        slot: u64,
        is_locked_out: bool,
    }

    fn simulate_fork_selection(
        neutral_fork: &ForkInfo,
        forks: &Vec<ForkInfo>,
        validators: &Vec<ValidatorInfo>,
    ) -> Vec<Option<ForkSelectionResponse>> {
        fn vote(bank: &Arc<Bank>, pubkey: &Pubkey, slot: Slot) {
            let mut vote_account = bank.get_account(&pubkey).unwrap();
            let mut vote_state = VoteState::from(&vote_account).unwrap();
            vote_state.process_slot_vote_unchecked(slot);
            let versioned = VoteStateVersions::Current(Box::new(vote_state));
            VoteState::to(&versioned, &mut vote_account).unwrap();
            bank.store_account(&pubkey, &vote_account);
        }

        let mut towers: Vec<Tower> = iter::repeat_with(|| Tower::new_for_tests(8, 0.67))
            .take(validators.len())
            .collect();

        for slot in &neutral_fork.fork {
            for tower in towers.iter_mut() {
                tower.record_bank_vote(Vote {
                    hash: Hash::default(),
                    slots: vec![*slot],
                    timestamp: None,
                });
            }
        }

        for fork_info in forks.iter() {
            for slot in fork_info.fork.iter() {
                for voter_index in fork_info.voters.iter() {
                    towers[*voter_index].record_bank_vote(Vote {
                        hash: Hash::default(),
                        slots: vec![*slot],
                        timestamp: None,
                    });
                }
            }
        }

        let genesis_vote_accounts: Vec<Account> = validators
            .iter()
            .map(|validator| {
                vote_state::create_account(
                    &validator.voting_keypair.pubkey(),
                    &validator.keypair.pubkey(),
                    0,
                    validator.stake,
                )
            })
            .collect();

        let genesis_stake_accounts: Vec<Account> = validators
            .iter()
            .enumerate()
            .map(|(i, validator)| {
                stake_state::create_account(
                    &validator.staking_keypair.pubkey(),
                    &validator.voting_keypair.pubkey(),
                    &genesis_vote_accounts[i],
                    &Rent::default(),
                    validator.stake,
                )
            })
            .collect();

        let mut genesis_config = create_genesis_config(10_000).genesis_config;
        genesis_config.accounts.clear();

        for i in 0..validators.len() {
            genesis_config.accounts.insert(
                validators[i].voting_keypair.pubkey(),
                genesis_vote_accounts[i].clone(),
            );
            genesis_config.accounts.insert(
                validators[i].staking_keypair.pubkey(),
                genesis_stake_accounts[i].clone(),
            );
        }

        let mut bank_forks = BankForks::new(neutral_fork.fork[0], Bank::new(&genesis_config));

        let mut fork_progresses: Vec<HashMap<u64, ForkProgress>> = iter::repeat_with(HashMap::new)
            .take(validators.len())
            .collect();

        for fork_progress in fork_progresses.iter_mut() {
            fork_progress
                .entry(neutral_fork.fork[0])
                .or_insert_with(|| ForkProgress::new(bank_forks.banks[&0].last_blockhash()));
        }

        for index in 1..neutral_fork.fork.len() {
            let bank = Bank::new_from_parent(
                &bank_forks.banks[&neutral_fork.fork[index - 1]].clone(),
                &validators[neutral_fork.leader].keypair.pubkey(),
                neutral_fork.fork[index],
            );

            bank_forks.insert(bank);

            for validator in validators.iter() {
                vote(
                    &bank_forks.banks[&neutral_fork.fork[index]].clone(),
                    &validator.voting_keypair.pubkey(),
                    neutral_fork.fork[index - 1],
                );
            }

            bank_forks.banks[&neutral_fork.fork[index]].freeze();

            for fork_progress in fork_progresses.iter_mut() {
                fork_progress
                    .entry(bank_forks.banks[&neutral_fork.fork[index]].slot())
                    .or_insert_with(|| {
                        ForkProgress::new(
                            bank_forks.banks[&neutral_fork.fork[index]].last_blockhash(),
                        )
                    });
            }
        }

        let last_neutral_bank = &bank_forks.banks[neutral_fork.fork.last().unwrap()].clone();

        for fork_info in forks.iter() {
            for index in 0..fork_info.fork.len() {
                let last_bank: &Arc<Bank>;
                let last_bank_in_fork: Arc<Bank>;

                if index == 0 {
                    last_bank = &last_neutral_bank;
                } else {
                    last_bank_in_fork = bank_forks.banks[&fork_info.fork[index - 1]].clone();
                    last_bank = &last_bank_in_fork;
                }

                let bank = Bank::new_from_parent(
                    last_bank,
                    &validators[fork_info.leader].keypair.pubkey(),
                    fork_info.fork[index],
                );

                bank_forks.insert(bank);

                for voter_index in fork_info.voters.iter() {
                    vote(
                        &bank_forks.banks[&fork_info.fork[index]].clone(),
                        &validators[*voter_index].voting_keypair.pubkey(),
                        last_bank.slot(),
                    );
                }

                bank_forks.banks[&fork_info.fork[index]].freeze();

                for fork_progress in fork_progresses.iter_mut() {
                    fork_progress
                        .entry(bank_forks.banks[&fork_info.fork[index]].slot())
                        .or_insert_with(|| {
                            ForkProgress::new(
                                bank_forks.banks[&fork_info.fork[index]].last_blockhash(),
                            )
                        });
                }
            }
        }

        let bank_fork_ancestors = bank_forks.ancestors();
        let wrapped_bank_fork = Arc::new(RwLock::new(bank_forks));

        (0..validators.len())
            .map(|i| {
                let mut frozen_banks: Vec<_> = wrapped_bank_fork
                    .read()
                    .unwrap()
                    .frozen_banks()
                    .values()
                    .cloned()
                    .collect();
                ReplayStage::compute_bank_stats(
                    &validators[i].keypair.pubkey(),
                    &bank_fork_ancestors,
                    &mut frozen_banks,
                    &towers[i],
                    &mut fork_progresses[i],
                );
                let (heaviest_bank, _) = ReplayStage::select_forks(
                    &frozen_banks,
                    &towers[i],
                    &mut fork_progresses[i],
                    &bank_fork_ancestors,
                );

                if heaviest_bank.is_none() {
                    None
                } else {
                    let bank = heaviest_bank.unwrap();
                    let stats = &fork_progresses[i].get(&bank.slot()).unwrap().fork_stats;
                    Some(ForkSelectionResponse {
                        slot: stats.slot,
                        is_locked_out: stats.is_locked_out,
                    })
                }
            })
            .collect()
    }

    #[test]
    fn test_minority_fork_overcommit_attack() {
        let neutral_fork = ForkInfo {
            leader: 0,
            fork: vec![0, 1, 2],
            voters: vec![],
        };

        let forks: Vec<ForkInfo> = vec![
            // Minority fork
            ForkInfo {
                leader: 2,
                fork: (3..=3 + 8).collect(),
                voters: vec![2],
            },
            ForkInfo {
                leader: 1,
                fork: (12..12 + 8).collect(),
                voters: vec![0, 1],
            },
        ];

        let validators: Vec<ValidatorInfo> = vec![
            ValidatorInfo {
                stake: 34_000_000,
                keypair: Keypair::new(),
                voting_keypair: Keypair::new(),
                staking_keypair: Keypair::new(),
            },
            ValidatorInfo {
                stake: 33_000_000,
                keypair: Keypair::new(),
                voting_keypair: Keypair::new(),
                staking_keypair: Keypair::new(),
            },
            // Malicious Node
            ValidatorInfo {
                stake: 33_000_000,
                keypair: Keypair::new(),
                voting_keypair: Keypair::new(),
                staking_keypair: Keypair::new(),
            },
        ];

        let resp = simulate_fork_selection(&neutral_fork, &forks, &validators);
        // Both honest nodes are now want to switch to minority fork and are locked out
        assert!(resp[0].is_some());
        assert_eq!(resp[0].as_ref().unwrap().is_locked_out, true);
        assert_eq!(
            resp[0].as_ref().unwrap().slot,
            forks[0].fork.last().unwrap().clone()
        );
        assert!(resp[1].is_some());
        assert_eq!(resp[1].as_ref().unwrap().is_locked_out, true);
        assert_eq!(
            resp[1].as_ref().unwrap().slot,
            forks[0].fork.last().unwrap().clone()
        );
    }

    #[test]
    fn test_child_slots_of_same_parent() {
        let ledger_path = get_tmp_ledger_path!();
        {
            let blockstore = Arc::new(
                Blockstore::open(&ledger_path)
                    .expect("Expected to be able to open database ledger"),
            );

            let genesis_config = create_genesis_config(10_000).genesis_config;
            let bank0 = Bank::new(&genesis_config);
            let leader_schedule_cache = Arc::new(LeaderScheduleCache::new_from_bank(&bank0));
            let exit = Arc::new(AtomicBool::new(false));
            let subscriptions = Arc::new(RpcSubscriptions::new(&exit));
            let bank_forks = BankForks::new(0, bank0);
            bank_forks.working_bank().freeze();

            // Insert shred for slot 1, generate new forks, check result
            let (shreds, _) = make_slot_entries(1, 0, 8);
            blockstore.insert_shreds(shreds, None, false).unwrap();
            assert!(bank_forks.get(1).is_none());
            let bank_forks = RwLock::new(bank_forks);
            ReplayStage::generate_new_bank_forks(
                &blockstore,
                &bank_forks,
                &leader_schedule_cache,
                &subscriptions,
                None,
            );
            assert!(bank_forks.read().unwrap().get(1).is_some());

            // Insert shred for slot 3, generate new forks, check result
            let (shreds, _) = make_slot_entries(2, 0, 8);
            blockstore.insert_shreds(shreds, None, false).unwrap();
            assert!(bank_forks.read().unwrap().get(2).is_none());
            ReplayStage::generate_new_bank_forks(
                &blockstore,
                &bank_forks,
                &leader_schedule_cache,
                &subscriptions,
                None,
            );
            assert!(bank_forks.read().unwrap().get(1).is_some());
            assert!(bank_forks.read().unwrap().get(2).is_some());
        }

        let _ignored = remove_dir_all(&ledger_path);
    }

    #[test]
    fn test_handle_new_root() {
        let genesis_config = create_genesis_config(10_000).genesis_config;
        let bank0 = Bank::new(&genesis_config);
        let bank_forks = Arc::new(RwLock::new(BankForks::new(0, bank0)));
        let root = 3;
        let root_bank = Bank::new_from_parent(
            bank_forks.read().unwrap().get(0).unwrap(),
            &Pubkey::default(),
            root,
        );
        bank_forks.write().unwrap().insert(root_bank);
        let mut progress = HashMap::new();
        for i in 0..=root {
            progress.insert(i, ForkProgress::new(Hash::default()));
        }
        let mut earliest_vote_on_fork = root - 1;
        ReplayStage::handle_new_root(
            root,
            &bank_forks,
            &mut progress,
            &None,
            &mut earliest_vote_on_fork,
        );
        assert_eq!(bank_forks.read().unwrap().root(), root);
        assert_eq!(progress.len(), 1);
        assert_eq!(earliest_vote_on_fork, root);
        assert!(progress.get(&root).is_some());

        earliest_vote_on_fork = root + 1;
        ReplayStage::handle_new_root(
            root,
            &bank_forks,
            &mut progress,
            &None,
            &mut earliest_vote_on_fork,
        );
        assert_eq!(earliest_vote_on_fork, root + 1);
    }

    #[test]
    fn test_dead_fork_transaction_error() {
        let keypair1 = Keypair::new();
        let keypair2 = Keypair::new();
        let missing_keypair = Keypair::new();
        let missing_keypair2 = Keypair::new();

        let res = check_dead_fork(|_keypair, bank| {
            let blockhash = bank.last_blockhash();
            let slot = bank.slot();
            let hashes_per_tick = bank.hashes_per_tick().unwrap_or(0);
            let entry = entry::next_entry(
                &blockhash,
                hashes_per_tick.saturating_sub(1),
                vec![
                    system_transaction::transfer(&keypair1, &keypair2.pubkey(), 2, blockhash), // should be fine,
                    system_transaction::transfer(
                        &missing_keypair,
                        &missing_keypair2.pubkey(),
                        2,
                        blockhash,
                    ), // should cause AccountNotFound error
                ],
            );
            entries_to_test_shreds(vec![entry], slot, slot.saturating_sub(1), false, 0)
        });

        assert_matches!(
            res,
            Err(BlockstoreProcessorError::InvalidTransaction(
                TransactionError::AccountNotFound
            ))
        );
    }

    #[test]
    fn test_dead_fork_entry_verification_failure() {
        let keypair2 = Keypair::new();
        let res = check_dead_fork(|genesis_keypair, bank| {
            let blockhash = bank.last_blockhash();
            let slot = bank.slot();
            let bad_hash = hash(&[2; 30]);
            let hashes_per_tick = bank.hashes_per_tick().unwrap_or(0);
            let entry = entry::next_entry(
                // Use wrong blockhash so that the entry causes an entry verification failure
                &bad_hash,
                hashes_per_tick.saturating_sub(1),
                vec![system_transaction::transfer(
                    &genesis_keypair,
                    &keypair2.pubkey(),
                    2,
                    blockhash,
                )],
            );
            entries_to_test_shreds(vec![entry], slot, slot.saturating_sub(1), false, 0)
        });

        if let Err(BlockstoreProcessorError::InvalidBlock(block_error)) = res {
            assert_eq!(block_error, BlockError::InvalidEntryHash);
        } else {
            assert!(false);
        }
    }

    #[test]
    fn test_dead_fork_invalid_tick_hash_count() {
        let res = check_dead_fork(|_keypair, bank| {
            let blockhash = bank.last_blockhash();
            let slot = bank.slot();
            let hashes_per_tick = bank.hashes_per_tick().unwrap_or(0);
            assert!(hashes_per_tick > 0);

            let too_few_hashes_tick = Entry::new(&blockhash, hashes_per_tick - 1, vec![]);
            entries_to_test_shreds(
                vec![too_few_hashes_tick],
                slot,
                slot.saturating_sub(1),
                false,
                0,
            )
        });

        if let Err(BlockstoreProcessorError::InvalidBlock(block_error)) = res {
            assert_eq!(block_error, BlockError::InvalidTickHashCount);
        } else {
            assert!(false);
        }
    }

    #[test]
    fn test_dead_fork_invalid_slot_tick_count() {
        // Too many ticks per slot
        let res = check_dead_fork(|_keypair, bank| {
            let blockhash = bank.last_blockhash();
            let slot = bank.slot();
            let hashes_per_tick = bank.hashes_per_tick().unwrap_or(0);
            entries_to_test_shreds(
                entry::create_ticks(bank.ticks_per_slot() + 1, hashes_per_tick, blockhash),
                slot,
                slot.saturating_sub(1),
                false,
                0,
            )
        });

        if let Err(BlockstoreProcessorError::InvalidBlock(block_error)) = res {
            assert_eq!(block_error, BlockError::InvalidTickCount);
        } else {
            assert!(false);
        }

        // Too few ticks per slot
        let res = check_dead_fork(|_keypair, bank| {
            let blockhash = bank.last_blockhash();
            let slot = bank.slot();
            let hashes_per_tick = bank.hashes_per_tick().unwrap_or(0);
            entries_to_test_shreds(
                entry::create_ticks(bank.ticks_per_slot() - 1, hashes_per_tick, blockhash),
                slot,
                slot.saturating_sub(1),
                true,
                0,
            )
        });

        if let Err(BlockstoreProcessorError::InvalidBlock(block_error)) = res {
            assert_eq!(block_error, BlockError::InvalidTickCount);
        } else {
            assert!(false);
        }
    }

    #[test]
    fn test_dead_fork_invalid_last_tick() {
        let res = check_dead_fork(|_keypair, bank| {
            let blockhash = bank.last_blockhash();
            let slot = bank.slot();
            let hashes_per_tick = bank.hashes_per_tick().unwrap_or(0);
            entries_to_test_shreds(
                entry::create_ticks(bank.ticks_per_slot(), hashes_per_tick, blockhash),
                slot,
                slot.saturating_sub(1),
                false,
                0,
            )
        });

        if let Err(BlockstoreProcessorError::InvalidBlock(block_error)) = res {
            assert_eq!(block_error, BlockError::InvalidLastTick);
        } else {
            assert!(false);
        }
    }

    #[test]
    fn test_dead_fork_trailing_entry() {
        let keypair = Keypair::new();
        let res = check_dead_fork(|genesis_keypair, bank| {
            let blockhash = bank.last_blockhash();
            let slot = bank.slot();
            let hashes_per_tick = bank.hashes_per_tick().unwrap_or(0);
            let mut entries =
                entry::create_ticks(bank.ticks_per_slot(), hashes_per_tick, blockhash.clone());
            let last_entry_hash = entries.last().unwrap().hash;
            let tx =
                system_transaction::transfer(&genesis_keypair, &keypair.pubkey(), 2, blockhash);
            let trailing_entry = entry::next_entry(&last_entry_hash, 1, vec![tx]);
            entries.push(trailing_entry);
            entries_to_test_shreds(entries, slot, slot.saturating_sub(1), true, 0)
        });

        if let Err(BlockstoreProcessorError::InvalidBlock(block_error)) = res {
            assert_eq!(block_error, BlockError::TrailingEntry);
        } else {
            assert!(false);
        }
    }

    #[test]
    fn test_dead_fork_entry_deserialize_failure() {
        // Insert entry that causes deserialization failure
        let res = check_dead_fork(|_, _| {
            let payload_len = SIZE_OF_DATA_SHRED_PAYLOAD;
            let gibberish = [0xa5u8; PACKET_DATA_SIZE];
            let mut data_header = DataShredHeader::default();
            data_header.flags |= DATA_COMPLETE_SHRED;
            let mut shred = Shred::new_empty_from_header(
                ShredCommonHeader::default(),
                data_header,
                CodingShredHeader::default(),
            );
            bincode::serialize_into(
                &mut shred.payload[SIZE_OF_COMMON_SHRED_HEADER + SIZE_OF_DATA_SHRED_HEADER..],
                &gibberish[..payload_len],
            )
            .unwrap();
            vec![shred]
        });

        assert_matches!(
            res,
            Err(
                BlockstoreProcessorError::FailedToLoadEntries(BlockstoreError::InvalidShredData(_)),
            )
        );
    }

    // Given a shred and a fatal expected error, check that replaying that shred causes causes the fork to be
    // marked as dead. Returns the error for caller to verify.
    fn check_dead_fork<F>(shred_to_insert: F) -> result::Result<(), BlockstoreProcessorError>
    where
        F: Fn(&Keypair, Arc<Bank>) -> Vec<Shred>,
    {
        let ledger_path = get_tmp_ledger_path!();
        let res = {
            let blockstore = Arc::new(
                Blockstore::open(&ledger_path)
                    .expect("Expected to be able to open database ledger"),
            );
            let GenesisConfigInfo {
                mut genesis_config,
                mint_keypair,
                ..
            } = create_genesis_config(1000);
            genesis_config.poh_config.hashes_per_tick = Some(2);
            let bank0 = Arc::new(Bank::new(&genesis_config));
            let mut progress = HashMap::new();
            let last_blockhash = bank0.last_blockhash();
            let mut bank0_progress = progress
                .entry(bank0.slot())
                .or_insert_with(|| ForkProgress::new(last_blockhash));
            let shreds = shred_to_insert(&mint_keypair, bank0.clone());
            blockstore.insert_shreds(shreds, None, false).unwrap();
            let res = ReplayStage::replay_blockstore_into_bank(
                &bank0,
                &blockstore,
                &mut bank0_progress,
                None,
                &VerifyRecyclers::default(),
            );

            // Check that the erroring bank was marked as dead in the progress map
            assert!(progress
                .get(&bank0.slot())
                .map(|b| b.is_dead)
                .unwrap_or(false));

            // Check that the erroring bank was marked as dead in blockstore
            assert!(blockstore.is_dead(bank0.slot()));
            res.map(|_| ())
        };
        let _ignored = remove_dir_all(&ledger_path);
        res
    }

    #[test]
    fn test_replay_commitment_cache() {
        fn leader_vote(bank: &Arc<Bank>, pubkey: &Pubkey) {
            let mut leader_vote_account = bank.get_account(&pubkey).unwrap();
            let mut vote_state = VoteState::from(&leader_vote_account).unwrap();
            vote_state.process_slot_vote_unchecked(bank.slot());
            let versioned = VoteStateVersions::Current(Box::new(vote_state));
            VoteState::to(&versioned, &mut leader_vote_account).unwrap();
            bank.store_account(&pubkey, &leader_vote_account);
        }

        let block_commitment_cache = Arc::new(RwLock::new(BlockCommitmentCache::default()));
        let (lockouts_sender, _) = AggregateCommitmentService::new(
            &Arc::new(AtomicBool::new(false)),
            block_commitment_cache.clone(),
        );

        let leader_pubkey = Pubkey::new_rand();
        let leader_lamports = 3;
        let genesis_config_info =
            create_genesis_config_with_leader(50, &leader_pubkey, leader_lamports);
        let mut genesis_config = genesis_config_info.genesis_config;
        let leader_voting_pubkey = genesis_config_info.voting_keypair.pubkey();
        genesis_config.epoch_schedule.warmup = false;
        genesis_config.ticks_per_slot = 4;
        let bank0 = Bank::new(&genesis_config);
        for _ in 0..genesis_config.ticks_per_slot {
            bank0.register_tick(&Hash::default());
        }
        bank0.freeze();
        let arc_bank0 = Arc::new(bank0);
        let bank_forks = Arc::new(RwLock::new(BankForks::new_from_banks(
            &[arc_bank0.clone()],
            vec![0],
        )));

        assert!(block_commitment_cache
            .read()
            .unwrap()
            .get_block_commitment(0)
            .is_none());
        assert!(block_commitment_cache
            .read()
            .unwrap()
            .get_block_commitment(1)
            .is_none());

        let bank1 = Bank::new_from_parent(&arc_bank0, &Pubkey::default(), arc_bank0.slot() + 1);
        let _res = bank1.transfer(10, &genesis_config_info.mint_keypair, &Pubkey::new_rand());
        for _ in 0..genesis_config.ticks_per_slot {
            bank1.register_tick(&Hash::default());
        }
        bank1.freeze();
        bank_forks.write().unwrap().insert(bank1);
        let arc_bank1 = bank_forks.read().unwrap().get(1).unwrap().clone();
        leader_vote(&arc_bank1, &leader_voting_pubkey);
        ReplayStage::update_commitment_cache(arc_bank1.clone(), leader_lamports, &lockouts_sender);

        let bank2 = Bank::new_from_parent(&arc_bank1, &Pubkey::default(), arc_bank1.slot() + 1);
        let _res = bank2.transfer(10, &genesis_config_info.mint_keypair, &Pubkey::new_rand());
        for _ in 0..genesis_config.ticks_per_slot {
            bank2.register_tick(&Hash::default());
        }
        bank2.freeze();
        bank_forks.write().unwrap().insert(bank2);
        let arc_bank2 = bank_forks.read().unwrap().get(2).unwrap().clone();
        leader_vote(&arc_bank2, &leader_voting_pubkey);
        ReplayStage::update_commitment_cache(arc_bank2.clone(), leader_lamports, &lockouts_sender);
        thread::sleep(Duration::from_millis(200));

        let mut expected0 = BlockCommitment::default();
        expected0.increase_confirmation_stake(2, leader_lamports);
        assert_eq!(
            block_commitment_cache
                .read()
                .unwrap()
                .get_block_commitment(0)
                .unwrap(),
            &expected0,
        );
        let mut expected1 = BlockCommitment::default();
        expected1.increase_confirmation_stake(2, leader_lamports);
        assert_eq!(
            block_commitment_cache
                .read()
                .unwrap()
                .get_block_commitment(1)
                .unwrap(),
            &expected1
        );
        let mut expected2 = BlockCommitment::default();
        expected2.increase_confirmation_stake(1, leader_lamports);
        assert_eq!(
            block_commitment_cache
                .read()
                .unwrap()
                .get_block_commitment(2)
                .unwrap(),
            &expected2
        );
    }

    pub fn create_test_transactions_and_populate_blockstore(
        keypairs: Vec<&Keypair>,
        previous_slot: Slot,
        bank: Arc<Bank>,
        blockstore: Arc<Blockstore>,
    ) -> Vec<Signature> {
        let mint_keypair = keypairs[0];
        let keypair1 = keypairs[1];
        let keypair2 = keypairs[2];
        let keypair3 = keypairs[3];
        let slot = bank.slot();
        let blockhash = bank.confirmed_last_blockhash().0;

        // Generate transactions for processing
        // Successful transaction
        let success_tx =
            system_transaction::transfer(&mint_keypair, &keypair1.pubkey(), 2, blockhash);
        let success_signature = success_tx.signatures[0];
        let entry_1 = next_entry(&blockhash, 1, vec![success_tx]);
        // Failed transaction, InstructionError
        let ix_error_tx =
            system_transaction::transfer(&keypair2, &keypair3.pubkey(), 10, blockhash);
        let ix_error_signature = ix_error_tx.signatures[0];
        let entry_2 = next_entry(&entry_1.hash, 1, vec![ix_error_tx]);
        // Failed transaction
        let fail_tx =
            system_transaction::transfer(&mint_keypair, &keypair2.pubkey(), 2, Hash::default());
        let entry_3 = next_entry(&entry_2.hash, 1, vec![fail_tx]);
        let entries = vec![entry_1, entry_2, entry_3];

        let shreds = entries_to_test_shreds(entries.clone(), slot, previous_slot, true, 0);
        blockstore.insert_shreds(shreds, None, false).unwrap();
        blockstore.set_roots(&[slot]).unwrap();

        let (transaction_status_sender, transaction_status_receiver) = unbounded();
        let transaction_status_service = TransactionStatusService::new(
            transaction_status_receiver,
            blockstore.clone(),
            &Arc::new(AtomicBool::new(false)),
        );

        // Check that process_entries successfully writes can_commit transactions statuses, and
        // that they are matched properly by get_confirmed_block
        let _result = blockstore_processor::process_entries(
            &bank,
            &entries,
            true,
            Some(transaction_status_sender),
        );

        transaction_status_service.join().unwrap();

        vec![success_signature, ix_error_signature]
    }

    #[test]
    fn test_write_persist_transaction_status() {
        let GenesisConfigInfo {
            genesis_config,
            mint_keypair,
            ..
        } = create_genesis_config(1000);
        let (ledger_path, _) = create_new_tmp_ledger!(&genesis_config);
        {
            let blockstore = Blockstore::open(&ledger_path)
                .expect("Expected to successfully open database ledger");
            let blockstore = Arc::new(blockstore);

            let keypair1 = Keypair::new();
            let keypair2 = Keypair::new();
            let keypair3 = Keypair::new();

            let bank0 = Arc::new(Bank::new(&genesis_config));
            bank0
                .transfer(4, &mint_keypair, &keypair2.pubkey())
                .unwrap();

            let bank1 = Arc::new(Bank::new_from_parent(&bank0, &Pubkey::default(), 1));
            let slot = bank1.slot();

            let signatures = create_test_transactions_and_populate_blockstore(
                vec![&mint_keypair, &keypair1, &keypair2, &keypair3],
                bank0.slot(),
                bank1,
                blockstore.clone(),
            );

            let confirmed_block = blockstore.get_confirmed_block(slot, None).unwrap();
            assert_eq!(confirmed_block.transactions.len(), 3);

            for RpcTransactionWithStatusMeta { transaction, meta } in
                confirmed_block.transactions.into_iter()
            {
                if let RpcEncodedTransaction::Json(transaction) = transaction {
                    if transaction.signatures[0] == signatures[0].to_string() {
                        assert_eq!(meta.unwrap().status, Ok(()));
                    } else if transaction.signatures[0] == signatures[1].to_string() {
                        assert_eq!(
                            meta.unwrap().status,
                            Err(TransactionError::InstructionError(
                                0,
                                InstructionError::CustomError(1)
                            ))
                        );
                    } else {
                        assert_eq!(meta, None);
                    }
                }
            }
        }
        Blockstore::destroy(&ledger_path).unwrap();
    }

    #[test]
    fn test_compute_bank_stats_confirmed() {
        let node_keypair = Keypair::new();
        let vote_keypair = Keypair::new();
        let stake_keypair = Keypair::new();
        let node_pubkey = node_keypair.pubkey();
        let mut keypairs = HashMap::new();
        keypairs.insert(
            node_pubkey,
            ValidatorVoteKeypairs::new(node_keypair, vote_keypair, stake_keypair),
        );

        let (bank_forks, mut progress) = initialize_state(&keypairs);
        let bank0 = bank_forks.get(0).unwrap().clone();
        let my_keypairs = keypairs.get(&node_pubkey).unwrap();
        let vote_tx = vote_transaction::new_vote_transaction(
            vec![0],
            bank0.hash(),
            bank0.last_blockhash(),
            &my_keypairs.node_keypair,
            &my_keypairs.vote_keypair,
            &my_keypairs.vote_keypair,
        );

        let bank_forks = RwLock::new(bank_forks);
        let bank1 = Bank::new_from_parent(&bank0, &node_pubkey, 1);
        bank1.process_transaction(&vote_tx).unwrap();
        bank1.freeze();

        // Test confirmations
        let ancestors = bank_forks.read().unwrap().ancestors();
        let mut frozen_banks: Vec<_> = bank_forks
            .read()
            .unwrap()
            .frozen_banks()
            .values()
            .cloned()
            .collect();
        let tower = Tower::new_for_tests(0, 0.67);
        let newly_computed = ReplayStage::compute_bank_stats(
            &node_pubkey,
            &ancestors,
            &mut frozen_banks,
            &tower,
            &mut progress,
        );
        assert_eq!(newly_computed, vec![0]);
        // The only vote is in bank 1, and bank_forks does not currently contain
        // bank 1, so no slot should be confirmed.
        {
            let fork_progress = progress.get(&0).unwrap();
            let confirmed_forks = ReplayStage::confirm_forks(
                &tower,
                &fork_progress.fork_stats.stake_lockouts,
                fork_progress.fork_stats.total_staked,
                &progress,
                &bank_forks,
            );

            assert!(confirmed_forks.is_empty())
        }

        // Insert the bank that contains a vote for slot 0, which confirms slot 0
        bank_forks.write().unwrap().insert(bank1);
        progress.insert(1, ForkProgress::new(bank0.last_blockhash()));
        let ancestors = bank_forks.read().unwrap().ancestors();
        let mut frozen_banks: Vec<_> = bank_forks
            .read()
            .unwrap()
            .frozen_banks()
            .values()
            .cloned()
            .collect();
        let newly_computed = ReplayStage::compute_bank_stats(
            &node_pubkey,
            &ancestors,
            &mut frozen_banks,
            &tower,
            &mut progress,
        );

        assert_eq!(newly_computed, vec![1]);
        {
            let fork_progress = progress.get(&1).unwrap();
            let confirmed_forks = ReplayStage::confirm_forks(
                &tower,
                &fork_progress.fork_stats.stake_lockouts,
                fork_progress.fork_stats.total_staked,
                &progress,
                &bank_forks,
            );
            assert_eq!(confirmed_forks, vec![0]);
        }

        let ancestors = bank_forks.read().unwrap().ancestors();
        let mut frozen_banks: Vec<_> = bank_forks
            .read()
            .unwrap()
            .frozen_banks()
            .values()
            .cloned()
            .collect();
        let newly_computed = ReplayStage::compute_bank_stats(
            &node_pubkey,
            &ancestors,
            &mut frozen_banks,
            &tower,
            &mut progress,
        );
        // No new stats should have been computed
        assert!(newly_computed.is_empty());
    }

    #[test]
    fn test_child_bank_heavier() {
        let node_keypair = Keypair::new();
        let vote_keypair = Keypair::new();
        let stake_keypair = Keypair::new();
        let node_pubkey = node_keypair.pubkey();
        let mut keypairs = HashMap::new();
        keypairs.insert(
            node_pubkey,
            ValidatorVoteKeypairs::new(node_keypair, vote_keypair, stake_keypair),
        );

        let (bank_forks, mut progress) = initialize_state(&keypairs);
        let bank_forks = Arc::new(RwLock::new(bank_forks));
        let mut tower = Tower::new_with_key(&node_pubkey);

        // Create the tree of banks in a BankForks object
        let forks = tr(0) / (tr(1) / (tr(2) / (tr(3))));

        let mut voting_simulator = VoteSimulator::new(&forks);
        let mut cluster_votes: HashMap<Pubkey, Vec<Slot>> = HashMap::new();
        let votes: Vec<Slot> = vec![0, 2];
        for vote in &votes {
            assert!(voting_simulator
                .simulate_vote(
                    *vote,
                    &bank_forks,
                    &mut cluster_votes,
                    &keypairs,
                    keypairs.get(&node_pubkey).unwrap(),
                    &mut progress,
                    &mut tower,
                )
                .is_empty());
        }

        let mut frozen_banks: Vec<_> = bank_forks
            .read()
            .unwrap()
            .frozen_banks()
            .values()
            .cloned()
            .collect();

        ReplayStage::compute_bank_stats(
            &Pubkey::default(),
            &bank_forks.read().unwrap().ancestors(),
            &mut frozen_banks,
            &tower,
            &mut progress,
        );

        frozen_banks.sort_by_key(|bank| bank.slot());
        for pair in frozen_banks.windows(2) {
            let first = progress
                .get(&pair[0].slot())
                .unwrap()
                .fork_stats
                .fork_weight;
            let second = progress
                .get(&pair[1].slot())
                .unwrap()
                .fork_stats
                .fork_weight;
            assert!(second >= first);
        }
    }
}
