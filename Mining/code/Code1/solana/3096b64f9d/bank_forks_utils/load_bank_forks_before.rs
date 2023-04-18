pub fn load_bank_forks(
    genesis_config: &GenesisConfig,
    blockstore: &Blockstore,
    account_paths: Vec<PathBuf>,
    shrink_paths: Option<Vec<PathBuf>>,
    snapshot_config: Option<&SnapshotConfig>,
    process_options: &ProcessOptions,
    cache_block_meta_sender: Option<&CacheBlockMetaSender>,
    accounts_update_notifier: Option<AccountsUpdateNotifier>,
) -> (
    Arc<RwLock<BankForks>>,
    LeaderScheduleCache,
    Option<StartingSnapshotHashes>,
) {
    let snapshot_present = if let Some(snapshot_config) = snapshot_config {
        info!(
            "Initializing bank snapshot path: {}",
            snapshot_config.bank_snapshots_dir.display()
        );
        let _ = fs::remove_dir_all(&snapshot_config.bank_snapshots_dir);
        fs::create_dir_all(&snapshot_config.bank_snapshots_dir)
            .expect("Couldn't create snapshot directory");

        if snapshot_utils::get_highest_full_snapshot_archive_info(
            &snapshot_config.full_snapshot_archives_dir,
        )
        .is_some()
        {
            true
        } else {
            info!("No snapshot package available; will load from genesis");
            false
        }
    } else {
        info!("Snapshots disabled; will load from genesis");
        false
    };

    let (bank_forks, starting_snapshot_hashes) = if snapshot_present {
        bank_forks_from_snapshot(
            genesis_config,
            account_paths,
            shrink_paths,
            snapshot_config.as_ref().unwrap(),
            process_options,
            accounts_update_notifier,
        )
    } else {
        let maybe_filler_accounts = process_options
            .accounts_db_config
            .as_ref()
            .map(|config| config.filler_accounts_config.count > 0);

        if let Some(true) = maybe_filler_accounts {
            panic!("filler accounts specified, but not loading from snapshot");
        }

        info!("Processing ledger from genesis");
        (
            blockstore_processor::process_blockstore_for_bank_0(
                genesis_config,
                blockstore,
                account_paths,
                process_options,
                cache_block_meta_sender,
                accounts_update_notifier,
            ),
            None,
        )
    };

    let mut leader_schedule_cache =
        LeaderScheduleCache::new_from_bank(&bank_forks.read().unwrap().root_bank());
    if process_options.full_leader_cache {
        leader_schedule_cache.set_max_schedules(std::usize::MAX);
    }

    if let Some(ref new_hard_forks) = process_options.new_hard_forks {
        let root_bank = bank_forks.read().unwrap().root_bank();
        let hard_forks = root_bank.hard_forks();

        for hard_fork_slot in new_hard_forks.iter() {
            if *hard_fork_slot > root_bank.slot() {
                hard_forks.write().unwrap().register(*hard_fork_slot);
            } else {
                warn!(
                    "Hard fork at {} ignored, --hard-fork option can be removed.",
                    hard_fork_slot
                );
            }
        }
    }

    (bank_forks, leader_schedule_cache, starting_snapshot_hashes)
}
