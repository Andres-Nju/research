    fn new(
        id: &'a Pubkey,
        vote_account: &'a Pubkey,
        start_progress: &'a Arc<RwLock<ValidatorStartProgress>>,
        blockstore: &'a Blockstore,
        original_blockstore_root: Slot,
        bank_forks: &'a Arc<RwLock<BankForks>>,
        leader_schedule_cache: &'a LeaderScheduleCache,
        process_options: &'a blockstore_processor::ProcessOptions,
        transaction_status_sender: Option<&'a TransactionStatusSender>,
        cache_block_meta_sender: Option<CacheBlockMetaSender>,
        blockstore_root_scan: BlockstoreRootScan,
        accounts_background_request_sender: AbsRequestSender,
        config: &'a ValidatorConfig,
    ) -> Self {
        Self {
            id,
            vote_account,
            start_progress,
            blockstore,
            original_blockstore_root,
            bank_forks,
            leader_schedule_cache,
            process_options,
            transaction_status_sender,
            cache_block_meta_sender,
            blockstore_root_scan: Some(blockstore_root_scan),
            accounts_background_request_sender,
            config,
            tower: None,
        }
    }

    pub(crate) fn process(&mut self) -> Result<(), String> {
        if self.tower.is_none() {
            let previous_start_process = *self.start_progress.read().unwrap();
            *self.start_progress.write().unwrap() = ValidatorStartProgress::LoadingLedger;

            let exit = Arc::new(AtomicBool::new(false));
            if let Ok(Some(max_slot)) = self.blockstore.highest_slot() {
                let bank_forks = self.bank_forks.clone();
                let exit = exit.clone();
                let start_progress = self.start_progress.clone();

                let _ = Builder::new()
                    .name("solRptLdgrStat".to_string())
                    .spawn(move || {
                        while !exit.load(Ordering::Relaxed) {
                            let slot = bank_forks.read().unwrap().working_bank().slot();
                            *start_progress.write().unwrap() =
                                ValidatorStartProgress::ProcessingLedger { slot, max_slot };
                            sleep(Duration::from_secs(2));
                        }
                    })
                    .unwrap();
            }
            if let Err(e) = blockstore_processor::process_blockstore_from_root(
                self.blockstore,
                self.bank_forks,
                self.leader_schedule_cache,
                self.process_options,
                self.transaction_status_sender,
                self.cache_block_meta_sender.as_ref(),
                &self.accounts_background_request_sender,
            ) {
                exit.store(true, Ordering::Relaxed);
                return Err(format!("Failed to load ledger: {e:?}"));
            }

            exit.store(true, Ordering::Relaxed);

            if let Some(blockstore_root_scan) = self.blockstore_root_scan.take() {
                blockstore_root_scan.join();
            }

            self.tower = Some({
                let restored_tower = Tower::restore(self.config.tower_storage.as_ref(), self.id);
                if let Ok(tower) = &restored_tower {
                    // reconciliation attempt 1 of 2 with tower
                    if let Err(e) = reconcile_blockstore_roots_with_external_source(
                        ExternalRootSource::Tower(tower.root()),
                        self.blockstore,
                        &mut self.original_blockstore_root,
                    ) {
                        return Err(format!("Failed to reconcile blockstore with tower: {e:?}"));
                    }
                }

                post_process_restored_tower(
                    restored_tower,
                    self.id,
                    self.vote_account,
                    self.config,
                    &self.bank_forks.read().unwrap(),
                )?
            });

            if let Some(hard_fork_restart_slot) = maybe_cluster_restart_with_hard_fork(
                self.config,
                self.bank_forks.read().unwrap().root_bank().slot(),
            ) {
                // reconciliation attempt 2 of 2 with hard fork
                // this should be #2 because hard fork root > tower root in almost all cases
                if let Err(e) = reconcile_blockstore_roots_with_external_source(
                    ExternalRootSource::HardFork(hard_fork_restart_slot),
                    self.blockstore,
                    &mut self.original_blockstore_root,
                ) {
                    return Err(format!(
                        "Failed to reconcile blockstore with hard fork: {e:?}"
                    ));
                }
            }

            *self.start_progress.write().unwrap() = previous_start_process;
        }
        Ok(())
    }

    pub(crate) fn process_to_create_tower(mut self) -> Result<Tower, String> {
        self.process()?;
        Ok(self.tower.unwrap())
    }
}

fn maybe_warp_slot(
    config: &ValidatorConfig,
    process_blockstore: &mut ProcessBlockStore,
    ledger_path: &Path,
    bank_forks: &RwLock<BankForks>,
    leader_schedule_cache: &LeaderScheduleCache,
    accounts_background_request_sender: &AbsRequestSender,
) -> Result<(), String> {
    if let Some(warp_slot) = config.warp_slot {
        process_blockstore.process()?;

        let mut bank_forks = bank_forks.write().unwrap();

        let working_bank = bank_forks.working_bank();

        if warp_slot <= working_bank.slot() {
            return Err(format!(
                "warp slot ({}) cannot be less than the working bank slot ({})",
                warp_slot,
                working_bank.slot()
            ));
        }
        info!("warping to slot {}", warp_slot);

        let root_bank = bank_forks.root_bank();
        bank_forks.insert(Bank::warp_from_parent(
            &root_bank,
            &Pubkey::default(),
            warp_slot,
        ));
        bank_forks.set_root(
            warp_slot,
            accounts_background_request_sender,
            Some(warp_slot),
        );
        leader_schedule_cache.set_root(&bank_forks.root_bank());

        let full_snapshot_archive_info = match snapshot_utils::bank_to_full_snapshot_archive(
            ledger_path,
            &bank_forks.root_bank(),
            None,
            &config.snapshot_config.full_snapshot_archives_dir,
            &config.snapshot_config.incremental_snapshot_archives_dir,
            config.snapshot_config.archive_format,
            config
                .snapshot_config
                .maximum_full_snapshot_archives_to_retain,
            config
                .snapshot_config
                .maximum_incremental_snapshot_archives_to_retain,
        ) {
            Ok(archive_info) => archive_info,
            Err(e) => return Err(format!("Unable to create snapshot: {e}")),
        };
        info!(
            "created snapshot: {}",
            full_snapshot_archive_info.path().display()
        );
    }
    Ok(())
}

fn blockstore_contains_bad_shred_version(
    blockstore: &Blockstore,
    start_slot: Slot,
    shred_version: u16,
) -> bool {
    let now = Instant::now();
    // Search for shreds with incompatible version in blockstore
    if let Ok(slot_meta_iterator) = blockstore.slot_meta_iterator(start_slot) {
        info!("Searching for incorrect shreds..");
        for (slot, _meta) in slot_meta_iterator {
            if let Ok(shreds) = blockstore.get_data_shreds_for_slot(slot, 0) {
                for shred in &shreds {
                    if shred.version() != shred_version {
                        return true;
                    }
                }
            }
            if now.elapsed().as_secs() > 60 {
                info!("Didn't find incorrect shreds after 60 seconds, aborting");
                return false;
            }
        }
    }
    false
}

fn backup_and_clear_blockstore(ledger_path: &Path, start_slot: Slot, shred_version: u16) {
    let blockstore = Blockstore::open(ledger_path).unwrap();
    let do_copy_and_clear =
        blockstore_contains_bad_shred_version(&blockstore, start_slot, shred_version);

    // If found, then copy shreds to another db and clear from start_slot
    if do_copy_and_clear {
        let folder_name = format!("backup_rocksdb_{}", thread_rng().gen_range(0, 99999));
        let backup_blockstore = Blockstore::open(&ledger_path.join(folder_name));
        let mut last_print = Instant::now();
        let mut copied = 0;
        let mut last_slot = None;
        let slot_meta_iterator = blockstore.slot_meta_iterator(start_slot).unwrap();
        for (slot, _meta) in slot_meta_iterator {
            if let Ok(shreds) = blockstore.get_data_shreds_for_slot(slot, 0) {
                if let Ok(ref backup_blockstore) = backup_blockstore {
                    copied += shreds.len();
                    let _ = backup_blockstore.insert_shreds(shreds, None, true);
                }
            }
            if last_print.elapsed().as_millis() > 3000 {
                info!(
                    "Copying shreds from slot {} copied {} so far.",
                    start_slot, copied
                );
                last_print = Instant::now();
            }
            last_slot = Some(slot);
        }

        let end_slot = last_slot.unwrap();
        info!("Purging slots {} to {}", start_slot, end_slot);
        blockstore.purge_from_next_slots(start_slot, end_slot);
        blockstore.purge_slots(start_slot, end_slot, PurgeType::Exact);
        info!("done");
    }
    drop(blockstore);
}

fn initialize_rpc_transaction_history_services(
    blockstore: Arc<Blockstore>,
    exit: &Arc<AtomicBool>,
    enable_rpc_transaction_history: bool,
    enable_extended_tx_metadata_storage: bool,
    transaction_notifier: Option<TransactionNotifierLock>,
) -> TransactionHistoryServices {
    let max_complete_transaction_status_slot = Arc::new(AtomicU64::new(blockstore.max_root()));
    let (transaction_status_sender, transaction_status_receiver) = unbounded();
    let transaction_status_sender = Some(TransactionStatusSender {
        sender: transaction_status_sender,
    });
    let transaction_status_service = Some(TransactionStatusService::new(
        transaction_status_receiver,
        max_complete_transaction_status_slot.clone(),
        enable_rpc_transaction_history,
        transaction_notifier.clone(),
        blockstore.clone(),
        enable_extended_tx_metadata_storage,
        exit,
    ));

    let (rewards_recorder_sender, rewards_receiver) = unbounded();
    let rewards_recorder_sender = Some(rewards_recorder_sender);
    let rewards_recorder_service = Some(RewardsRecorderService::new(
        rewards_receiver,
        blockstore.clone(),
        exit,
    ));

    let (cache_block_meta_sender, cache_block_meta_receiver) = unbounded();
    let cache_block_meta_sender = Some(cache_block_meta_sender);
    let cache_block_meta_service = Some(CacheBlockMetaService::new(
        cache_block_meta_receiver,
        blockstore,
        exit,
    ));
    TransactionHistoryServices {
        transaction_status_sender,
        transaction_status_service,
        max_complete_transaction_status_slot,
        rewards_recorder_sender,
        rewards_recorder_service,
        cache_block_meta_sender,
        cache_block_meta_service,
    }
}

#[derive(Debug, PartialEq, Eq)]
enum ValidatorError {
    BadExpectedBankHash,
    NotEnoughLedgerData,
    Error(String),
}

// Return if the validator waited on other nodes to start. In this case
// it should not wait for one of it's votes to land to produce blocks
// because if the whole network is waiting, then it will stall.
//
// Error indicates that a bad hash was encountered or another condition
// that is unrecoverable and the validator should exit.
fn wait_for_supermajority(
    config: &ValidatorConfig,
    process_blockstore: Option<&mut ProcessBlockStore>,
    bank_forks: &RwLock<BankForks>,
    cluster_info: &ClusterInfo,
    rpc_override_health_check: Arc<AtomicBool>,
    start_progress: &Arc<RwLock<ValidatorStartProgress>>,
) -> Result<bool, ValidatorError> {
    match config.wait_for_supermajority {
        None => Ok(false),
        Some(wait_for_supermajority_slot) => {
            if let Some(process_blockstore) = process_blockstore {
                process_blockstore
                    .process()
                    .map_err(ValidatorError::Error)?;
            }

            let bank = bank_forks.read().unwrap().working_bank();
            match wait_for_supermajority_slot.cmp(&bank.slot()) {
                std::cmp::Ordering::Less => return Ok(false),
                std::cmp::Ordering::Greater => {
                    error!(
                        "Ledger does not have enough data to wait for supermajority, \
                             please enable snapshot fetch. Has {} needs {}",
                        bank.slot(),
                        wait_for_supermajority_slot
                    );
                    return Err(ValidatorError::NotEnoughLedgerData);
                }
                _ => {}
            }

            if let Some(expected_bank_hash) = config.expected_bank_hash {
                if bank.hash() != expected_bank_hash {
                    error!(
                        "Bank hash({}) does not match expected value: {}",
                        bank.hash(),
                        expected_bank_hash
                    );
                    return Err(ValidatorError::BadExpectedBankHash);
                }
            }

            for i in 1.. {
                if i % 10 == 1 {
                    info!(
                        "Waiting for {}% of activated stake at slot {} to be in gossip...",
                        WAIT_FOR_SUPERMAJORITY_THRESHOLD_PERCENT,
                        bank.slot()
                    );
                }

                let gossip_stake_percent =
                    get_stake_percent_in_gossip(&bank, cluster_info, i % 10 == 0);

                *start_progress.write().unwrap() =
                    ValidatorStartProgress::WaitingForSupermajority {
                        slot: wait_for_supermajority_slot,
                        gossip_stake_percent,
                    };

                if gossip_stake_percent >= WAIT_FOR_SUPERMAJORITY_THRESHOLD_PERCENT {
                    info!(
                        "Supermajority reached, {}% active stake detected, starting up now.",
                        gossip_stake_percent,
                    );
                    break;
                }
                // The normal RPC health checks don't apply as the node is waiting, so feign health to
                // prevent load balancers from removing the node from their list of candidates during a
                // manual restart.
                rpc_override_health_check.store(true, Ordering::Relaxed);
                sleep(Duration::new(1, 0));
            }
            rpc_override_health_check.store(false, Ordering::Relaxed);
            Ok(true)
        }
    }
}

// Get the activated stake percentage (based on the provided bank) that is visible in gossip
fn get_stake_percent_in_gossip(bank: &Bank, cluster_info: &ClusterInfo, log: bool) -> u64 {
    let mut online_stake = 0;
    let mut wrong_shred_stake = 0;
    let mut wrong_shred_nodes = vec![];
    let mut offline_stake = 0;
    let mut offline_nodes = vec![];

    let mut total_activated_stake = 0;
    let now = timestamp();
    // Nodes contact infos are saved to disk and restored on validator startup.
    // Staked nodes entries will not expire until an epoch after. So it
    // is necessary here to filter for recent entries to establish liveness.
    let peers: HashMap<_, _> = cluster_info
        .all_tvu_peers()
        .into_iter()
        .filter(|node| {
            let age = now.saturating_sub(node.wallclock);
            // Contact infos are refreshed twice during this period.
            age < CRDS_GOSSIP_PULL_CRDS_TIMEOUT_MS
        })
        .map(|node| (node.id, node))
        .collect();
    let my_shred_version = cluster_info.my_shred_version();
    let my_id = cluster_info.id();

    for (activated_stake, vote_account) in bank.vote_accounts().values() {
        let activated_stake = *activated_stake;
        total_activated_stake += activated_stake;

        if activated_stake == 0 {
            continue;
        }
        let vote_state_node_pubkey = vote_account.node_pubkey().unwrap_or_default();

        if let Some(peer) = peers.get(&vote_state_node_pubkey) {
            if peer.shred_version == my_shred_version {
                trace!(
                    "observed {} in gossip, (activated_stake={})",
                    vote_state_node_pubkey,
                    activated_stake
                );
                online_stake += activated_stake;
            } else {
                wrong_shred_stake += activated_stake;
                wrong_shred_nodes.push((activated_stake, vote_state_node_pubkey));
            }
        } else if vote_state_node_pubkey == my_id {
            online_stake += activated_stake; // This node is online
        } else {
            offline_stake += activated_stake;
            offline_nodes.push((activated_stake, vote_state_node_pubkey));
        }
    }

    let online_stake_percentage = (online_stake as f64 / total_activated_stake as f64) * 100.;
    if log {
        info!(
            "{:.3}% of active stake visible in gossip",
            online_stake_percentage
        );

        if !wrong_shred_nodes.is_empty() {
            info!(
                "{:.3}% of active stake has the wrong shred version in gossip",
                (wrong_shred_stake as f64 / total_activated_stake as f64) * 100.,
            );
            wrong_shred_nodes.sort_by(|b, a| a.0.cmp(&b.0)); // sort by reverse stake weight
            for (stake, identity) in wrong_shred_nodes {
                info!(
                    "    {:.3}% - {}",
                    (stake as f64 / total_activated_stake as f64) * 100.,
                    identity
                );
            }
        }

        if !offline_nodes.is_empty() {
            info!(
                "{:.3}% of active stake is not visible in gossip",
                (offline_stake as f64 / total_activated_stake as f64) * 100.
            );
            offline_nodes.sort_by(|b, a| a.0.cmp(&b.0)); // sort by reverse stake weight
            for (stake, identity) in offline_nodes {
                info!(
                    "    {:.3}% - {}",
                    (stake as f64 / total_activated_stake as f64) * 100.,
                    identity
                );
            }
        }
    }

    online_stake_percentage as u64
}

/// Delete directories/files asynchronously to avoid blocking on it.
/// Fist, in sync context, rename the original path to *_deleted,
/// then spawn a thread to delete the renamed path.
/// If the process is killed and the deleting process is not done,
/// the leftover path will be deleted in the next process life, so
/// there is no file space leaking.
pub fn move_and_async_delete_path(path: impl AsRef<Path> + Copy) {
    let mut path_delete = PathBuf::new();
    path_delete.push(path);
    path_delete.set_file_name(format!(
        "{}{}",
        path_delete.file_name().unwrap().to_str().unwrap(),
        "_to_be_deleted"
    ));

    if path_delete.exists() {
        std::fs::remove_dir_all(&path_delete).unwrap();
    }

    if !path.as_ref().exists() {
        return;
    }

    if let Err(err) = std::fs::rename(path, &path_delete) {
        warn!(
            "Path renaming failed: {}.  Falling back to rm_dir in sync mode",
            err.to_string()
        );
        delete_contents_of_path(path);
        return;
    }

    Builder::new()
        .name("solDeletePath".to_string())
        .spawn(move || {
            std::fs::remove_dir_all(path_delete).unwrap();
        })
        .unwrap();
}

/// Delete the files and subdirectories in a directory.
/// This is useful if the process does not have permission
/// to delete the top level directory it might be able to
/// delete the contents of that directory.
fn delete_contents_of_path(path: impl AsRef<Path> + Copy) {
    if let Ok(dir_entries) = std::fs::read_dir(path) {
        for entry in dir_entries.flatten() {
            let sub_path = entry.path();
            let metadata = match entry.metadata() {
                Ok(metadata) => metadata,
                Err(err) => {
                    warn!(
                        "Failed to get metadata for {}. Error: {}",
                        sub_path.display(),
                        err.to_string()
                    );
                    break;
                }
            };
            if metadata.is_dir() {
                if let Err(err) = std::fs::remove_dir_all(&sub_path) {
                    warn!(
                        "Failed to remove sub directory {}.  Error: {}",
                        sub_path.display(),
                        err.to_string()
                    );
                }
            } else if metadata.is_file() {
                if let Err(err) = std::fs::remove_file(&sub_path) {
                    warn!(
                        "Failed to remove file {}.  Error: {}",
                        sub_path.display(),
                        err.to_string()
                    );
                }
            }
        }
    } else {
        warn!(
            "Failed to read the sub paths of {}",
            path.as_ref().display()
        );
    }
}

fn cleanup_accounts_paths(config: &ValidatorConfig) {
    for accounts_path in &config.account_paths {
        move_and_async_delete_path(accounts_path);
    }
    if let Some(ref shrink_paths) = config.account_shrink_paths {
        for accounts_path in shrink_paths {
            move_and_async_delete_path(accounts_path);
        }
    }
}

pub fn is_snapshot_config_valid(
    snapshot_config: &SnapshotConfig,
    accounts_hash_interval_slots: Slot,
) -> bool {
    // if the snapshot config is configured to *not* take snapshots, then it is valid
    if !snapshot_config.should_generate_snapshots() {
        return true;
    }

    let full_snapshot_interval_slots = snapshot_config.full_snapshot_archive_interval_slots;
    let incremental_snapshot_interval_slots =
        snapshot_config.incremental_snapshot_archive_interval_slots;

    let is_incremental_config_valid = if incremental_snapshot_interval_slots == Slot::MAX {
        true
    } else {
        incremental_snapshot_interval_slots >= accounts_hash_interval_slots
            && incremental_snapshot_interval_slots % accounts_hash_interval_slots == 0
            && full_snapshot_interval_slots > incremental_snapshot_interval_slots
    };

    full_snapshot_interval_slots >= accounts_hash_interval_slots
        && full_snapshot_interval_slots % accounts_hash_interval_slots == 0
        && is_incremental_config_valid
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crossbeam_channel::{bounded, RecvTimeoutError},
        solana_ledger::{create_new_tmp_ledger, genesis_utils::create_genesis_config_with_leader},
        solana_sdk::{genesis_config::create_genesis_config, poh_config::PohConfig},
        solana_tpu_client::tpu_connection_cache::{
            DEFAULT_TPU_CONNECTION_POOL_SIZE, DEFAULT_TPU_ENABLE_UDP, DEFAULT_TPU_USE_QUIC,
        },
        std::{fs::remove_dir_all, thread, time::Duration},
    };

    #[test]
    fn validator_exit() {
        solana_logger::setup();
        let leader_keypair = Keypair::new();
        let leader_node = Node::new_localhost_with_pubkey(&leader_keypair.pubkey());

        let validator_keypair = Keypair::new();
        let validator_node = Node::new_localhost_with_pubkey(&validator_keypair.pubkey());
        let genesis_config =
            create_genesis_config_with_leader(10_000, &leader_keypair.pubkey(), 1000)
                .genesis_config;
        let (validator_ledger_path, _blockhash) = create_new_tmp_ledger!(&genesis_config);

        let voting_keypair = Arc::new(Keypair::new());
        let config = ValidatorConfig {
            rpc_addrs: Some((validator_node.info.rpc, validator_node.info.rpc_pubsub)),
            ..ValidatorConfig::default_for_test()
        };
        let start_progress = Arc::new(RwLock::new(ValidatorStartProgress::default()));
        let validator = Validator::new(
            validator_node,
            Arc::new(validator_keypair),
            &validator_ledger_path,
            &voting_keypair.pubkey(),
            Arc::new(RwLock::new(vec![voting_keypair.clone()])),
            vec![leader_node.info],
            &config,
            true, // should_check_duplicate_instance
            start_progress.clone(),
            SocketAddrSpace::Unspecified,
            DEFAULT_TPU_USE_QUIC,
            DEFAULT_TPU_CONNECTION_POOL_SIZE,
            DEFAULT_TPU_ENABLE_UDP,
        )
        .expect("assume successful validator start");
        assert_eq!(
            *start_progress.read().unwrap(),
            ValidatorStartProgress::Running
        );
        validator.close();
        remove_dir_all(validator_ledger_path).unwrap();
    }

    #[test]
    fn test_backup_and_clear_blockstore() {
        use std::time::Instant;
        solana_logger::setup();
        use {
            solana_entry::entry,
            solana_ledger::{blockstore, get_tmp_ledger_path},
        };
        let blockstore_path = get_tmp_ledger_path!();
        {
            let blockstore = Blockstore::open(&blockstore_path).unwrap();

            let entries = entry::create_ticks(1, 0, Hash::default());

            info!("creating shreds");
            let mut last_print = Instant::now();
            for i in 1..10 {
                let shreds = blockstore::entries_to_test_shreds(
                    &entries,
                    i,     // slot
                    i - 1, // parent_slot
                    true,  // is_full_slot
                    1,     // version
                    true,  // merkle_variant
                );
                blockstore.insert_shreds(shreds, None, true).unwrap();
                if last_print.elapsed().as_millis() > 5000 {
                    info!("inserted {}", i);
                    last_print = Instant::now();
                }
            }
            drop(blockstore);

            // this purges and compacts all slots greater than or equal to 5
            backup_and_clear_blockstore(&blockstore_path, 5, 2);

            let blockstore = Blockstore::open(&blockstore_path).unwrap();
            // assert that slots less than 5 aren't affected
            assert!(blockstore.meta(4).unwrap().unwrap().next_slots.is_empty());
            for i in 5..10 {
                assert!(blockstore
                    .get_data_shreds_for_slot(i, 0)
                    .unwrap()
                    .is_empty());
            }
        }
    }

    #[test]
    fn validator_parallel_exit() {
        let leader_keypair = Keypair::new();
        let leader_node = Node::new_localhost_with_pubkey(&leader_keypair.pubkey());

        let mut ledger_paths = vec![];
        let mut validators: Vec<Validator> = (0..2)
            .map(|_| {
                let validator_keypair = Keypair::new();
                let validator_node = Node::new_localhost_with_pubkey(&validator_keypair.pubkey());
                let genesis_config =
                    create_genesis_config_with_leader(10_000, &leader_keypair.pubkey(), 1000)
                        .genesis_config;
                let (validator_ledger_path, _blockhash) = create_new_tmp_ledger!(&genesis_config);
                ledger_paths.push(validator_ledger_path.clone());
                let vote_account_keypair = Keypair::new();
                let config = ValidatorConfig {
                    rpc_addrs: Some((validator_node.info.rpc, validator_node.info.rpc_pubsub)),
                    ..ValidatorConfig::default_for_test()
                };
                Validator::new(
                    validator_node,
                    Arc::new(validator_keypair),
                    &validator_ledger_path,
                    &vote_account_keypair.pubkey(),
                    Arc::new(RwLock::new(vec![Arc::new(vote_account_keypair)])),
                    vec![leader_node.info.clone()],
                    &config,
                    true, // should_check_duplicate_instance
                    Arc::new(RwLock::new(ValidatorStartProgress::default())),
                    SocketAddrSpace::Unspecified,
                    DEFAULT_TPU_USE_QUIC,
                    DEFAULT_TPU_CONNECTION_POOL_SIZE,
                    DEFAULT_TPU_ENABLE_UDP,
                )
                .expect("assume successful validator start")
            })
            .collect();

        // Each validator can exit in parallel to speed many sequential calls to join`
        validators.iter_mut().for_each(|v| v.exit());

        // spawn a new thread to wait for the join of the validator
        let (sender, receiver) = bounded(0);
        let _ = thread::spawn(move || {
            validators.into_iter().for_each(|validator| {
                validator.join();
            });
            sender.send(()).unwrap();
        });

        // timeout of 30s for shutting down the validators
        let timeout = Duration::from_secs(30);
        if let Err(RecvTimeoutError::Timeout) = receiver.recv_timeout(timeout) {
            panic!("timeout for shutting down validators",);
        }

        for path in ledger_paths {
            remove_dir_all(path).unwrap();
        }
