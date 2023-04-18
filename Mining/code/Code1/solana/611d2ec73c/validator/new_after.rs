    pub fn new(
        mut node: Node,
        identity_keypair: Arc<Keypair>,
        ledger_path: &Path,
        vote_account: &Pubkey,
        authorized_voter_keypairs: Arc<RwLock<Vec<Arc<Keypair>>>>,
        cluster_entrypoints: Vec<ContactInfo>,
        config: &ValidatorConfig,
        should_check_duplicate_instance: bool,
        start_progress: Arc<RwLock<ValidatorStartProgress>>,
        socket_addr_space: SocketAddrSpace,
        use_quic: bool,
        tpu_connection_pool_size: usize,
    ) -> Self {
        let id = identity_keypair.pubkey();
        assert_eq!(id, node.info.id);

        warn!("identity: {}", id);
        warn!("vote account: {}", vote_account);

        if !config.no_os_network_stats_reporting {
            verify_udp_stats_access().unwrap_or_else(|err| {
                error!("Failed to access UDP stats: {}. Bypass check with --no-os-network-stats-reporting.", err);
                abort();
            });
        }

        let mut bank_notification_senders = Vec::new();

        let geyser_plugin_service =
            if let Some(geyser_plugin_config_files) = &config.geyser_plugin_config_files {
                let (confirmed_bank_sender, confirmed_bank_receiver) = unbounded();
                bank_notification_senders.push(confirmed_bank_sender);
                let result =
                    GeyserPluginService::new(confirmed_bank_receiver, geyser_plugin_config_files);
                match result {
                    Ok(geyser_plugin_service) => Some(geyser_plugin_service),
                    Err(err) => {
                        error!("Failed to load the Geyser plugin: {:?}", err);
                        abort();
                    }
                }
            } else {
                None
            };

        if config.voting_disabled {
            warn!("voting disabled");
            authorized_voter_keypairs.write().unwrap().clear();
        } else {
            for authorized_voter_keypair in authorized_voter_keypairs.read().unwrap().iter() {
                warn!("authorized voter: {}", authorized_voter_keypair.pubkey());
            }
        }

        for cluster_entrypoint in &cluster_entrypoints {
            info!("entrypoint: {:?}", cluster_entrypoint);
        }

        if solana_perf::perf_libs::api().is_some() {
            info!("Initializing sigverify, this could take a while...");
        } else {
            info!("Initializing sigverify...");
        }
        sigverify::init();
        info!("Done.");

        if !ledger_path.is_dir() {
            error!(
                "ledger directory does not exist or is not accessible: {:?}",
                ledger_path
            );
            abort();
        }

        if let Some(shred_version) = config.expected_shred_version {
            if let Some(wait_for_supermajority_slot) = config.wait_for_supermajority {
                *start_progress.write().unwrap() = ValidatorStartProgress::CleaningBlockStore;
                backup_and_clear_blockstore(
                    ledger_path,
                    wait_for_supermajority_slot + 1,
                    shred_version,
                );
            }
        }

        info!("Cleaning accounts paths..");
        *start_progress.write().unwrap() = ValidatorStartProgress::CleaningAccounts;
        let mut start = Measure::start("clean_accounts_paths");
        for accounts_path in &config.account_paths {
            cleanup_accounts_path(accounts_path);
        }
        if let Some(ref shrink_paths) = config.account_shrink_paths {
            for accounts_path in shrink_paths {
                cleanup_accounts_path(accounts_path);
            }
        }
        start.stop();
        info!("done. {}", start);

        let exit = Arc::new(AtomicBool::new(false));
        {
            let exit = exit.clone();
            config
                .validator_exit
                .write()
                .unwrap()
                .register_exit(Box::new(move || exit.store(true, Ordering::Relaxed)));
        }

        let accounts_update_notifier = geyser_plugin_service
            .as_ref()
            .and_then(|geyser_plugin_service| geyser_plugin_service.get_accounts_update_notifier());

        let transaction_notifier = geyser_plugin_service
            .as_ref()
            .and_then(|geyser_plugin_service| geyser_plugin_service.get_transaction_notifier());

        let block_metadata_notifier = geyser_plugin_service
            .as_ref()
            .and_then(|geyser_plugin_service| geyser_plugin_service.get_block_metadata_notifier());

        info!(
            "Geyser plugin: accounts_update_notifier: {} transaction_notifier: {}",
            accounts_update_notifier.is_some(),
            transaction_notifier.is_some()
        );

        let system_monitor_service = Some(SystemMonitorService::new(
            Arc::clone(&exit),
            !config.no_os_memory_stats_reporting,
            !config.no_os_network_stats_reporting,
            !config.no_os_cpu_stats_reporting,
        ));

        let (poh_timing_point_sender, poh_timing_point_receiver) = unbounded();
        let poh_timing_report_service =
            PohTimingReportService::new(poh_timing_point_receiver, &exit);

        let (
            genesis_config,
            bank_forks,
            blockstore,
            ledger_signal_receiver,
            completed_slots_receiver,
            leader_schedule_cache,
            starting_snapshot_hashes,
            TransactionHistoryServices {
                transaction_status_sender,
                transaction_status_service,
                max_complete_transaction_status_slot,
                rewards_recorder_sender,
                rewards_recorder_service,
                cache_block_meta_sender,
                cache_block_meta_service,
            },
            blockstore_process_options,
            blockstore_root_scan,
            pruned_banks_receiver,
        ) = load_blockstore(
            config,
            ledger_path,
            &exit,
            &start_progress,
            accounts_update_notifier,
            transaction_notifier,
            Some(poh_timing_point_sender.clone()),
        );

        node.info.wallclock = timestamp();
        node.info.shred_version = compute_shred_version(
            &genesis_config.hash(),
            Some(
                &bank_forks
                    .read()
                    .unwrap()
                    .working_bank()
                    .hard_forks()
                    .read()
                    .unwrap(),
            ),
        );

        Self::print_node_info(&node);

        if let Some(expected_shred_version) = config.expected_shred_version {
            if expected_shred_version != node.info.shred_version {
                error!(
                    "shred version mismatch: expected {} found: {}",
                    expected_shred_version, node.info.shred_version,
                );
                abort();
            }
        }

        let mut cluster_info = ClusterInfo::new(
            node.info.clone(),
            identity_keypair.clone(),
            socket_addr_space,
        );
        cluster_info.set_contact_debug_interval(config.contact_debug_interval);
        cluster_info.set_entrypoints(cluster_entrypoints);
        cluster_info.restore_contact_info(ledger_path, config.contact_save_interval);
        let cluster_info = Arc::new(cluster_info);

        let (
            accounts_background_service,
            accounts_hash_verifier,
            snapshot_packager_service,
            accounts_background_request_sender,
        ) = {
            let pending_accounts_package = PendingAccountsPackage::default();
            let (
                accounts_background_request_sender,
                snapshot_request_handler,
                pending_snapshot_package,
                snapshot_packager_service,
            ) = if let Some(snapshot_config) = config.snapshot_config.clone() {
                if !is_snapshot_config_valid(
                    snapshot_config.full_snapshot_archive_interval_slots,
                    snapshot_config.incremental_snapshot_archive_interval_slots,
                    config.accounts_hash_interval_slots,
                ) {
                    error!("Snapshot config is invalid");
                }

                let pending_snapshot_package = PendingSnapshotPackage::default();

                // filler accounts make snapshots invalid for use
                // so, do not publish that we have snapshots
                let enable_gossip_push = config
                    .accounts_db_config
                    .as_ref()
                    .map(|config| config.filler_accounts_config.count == 0)
                    .unwrap_or(true);

                let snapshot_packager_service = SnapshotPackagerService::new(
                    pending_snapshot_package.clone(),
                    starting_snapshot_hashes,
                    &exit,
                    &cluster_info,
                    snapshot_config.clone(),
                    enable_gossip_push,
                );

                let (snapshot_request_sender, snapshot_request_receiver) = unbounded();
                (
                    AbsRequestSender::new(snapshot_request_sender),
                    Some(SnapshotRequestHandler {
                        snapshot_config,
                        snapshot_request_receiver,
                        pending_accounts_package: pending_accounts_package.clone(),
                    }),
                    Some(pending_snapshot_package),
                    Some(snapshot_packager_service),
                )
            } else {
                (AbsRequestSender::default(), None, None, None)
            };

            let accounts_hash_verifier = AccountsHashVerifier::new(
                Arc::clone(&pending_accounts_package),
                pending_snapshot_package,
                &exit,
                &cluster_info,
                config.known_validators.clone(),
                config.halt_on_known_validators_accounts_hash_mismatch,
                config.accounts_hash_fault_injection_slots,
                config.snapshot_config.clone(),
            );

            let last_full_snapshot_slot = starting_snapshot_hashes.map(|x| x.full.hash.0);
            let accounts_background_service = AccountsBackgroundService::new(
                bank_forks.clone(),
                &exit,
                AbsRequestHandler {
                    snapshot_request_handler,
                    pruned_banks_receiver,
                },
                config.accounts_db_caching_enabled,
                config.accounts_db_test_hash_calculation,
                last_full_snapshot_slot,
            );

            (
                accounts_background_service,
                accounts_hash_verifier,
                snapshot_packager_service,
                accounts_background_request_sender,
            )
        };

        let leader_schedule_cache = Arc::new(leader_schedule_cache);
        let mut process_blockstore = ProcessBlockStore::new(
            &id,
            vote_account,
            &start_progress,
            &blockstore,
            &bank_forks,
            &leader_schedule_cache,
            &blockstore_process_options,
            transaction_status_sender.as_ref(),
            cache_block_meta_sender.clone(),
            blockstore_root_scan,
            accounts_background_request_sender.clone(),
            config,
        );

        maybe_warp_slot(
            config,
            &mut process_blockstore,
            ledger_path,
            &bank_forks,
            &leader_schedule_cache,
        );

        *start_progress.write().unwrap() = ValidatorStartProgress::StartingServices;

        let sample_performance_service =
            if config.rpc_addrs.is_some() && config.rpc_config.enable_rpc_transaction_history {
                Some(SamplePerformanceService::new(
                    &bank_forks,
                    &blockstore,
                    &exit,
                ))
            } else {
                None
            };

        let mut block_commitment_cache = BlockCommitmentCache::default();
        let bank_forks_guard = bank_forks.read().unwrap();
        block_commitment_cache.initialize_slots(
            bank_forks_guard.working_bank().slot(),
            bank_forks_guard.root(),
        );
        drop(bank_forks_guard);
        let block_commitment_cache = Arc::new(RwLock::new(block_commitment_cache));

        let optimistically_confirmed_bank =
            OptimisticallyConfirmedBank::locked_from_bank_forks_root(&bank_forks);

        let rpc_subscriptions = Arc::new(RpcSubscriptions::new_with_config(
            &exit,
            max_complete_transaction_status_slot.clone(),
            blockstore.clone(),
            bank_forks.clone(),
            block_commitment_cache.clone(),
            optimistically_confirmed_bank.clone(),
            &config.pubsub_config,
        ));

        let max_slots = Arc::new(MaxSlots::default());
        let (completed_data_sets_sender, completed_data_sets_receiver) =
            bounded(MAX_COMPLETED_DATA_SETS_IN_CHANNEL);
        let completed_data_sets_service = CompletedDataSetsService::new(
            completed_data_sets_receiver,
            blockstore.clone(),
            rpc_subscriptions.clone(),
            &exit,
            max_slots.clone(),
        );

        let poh_config = Arc::new(genesis_config.poh_config.clone());
        let (poh_recorder, entry_receiver, record_receiver) = {
            let bank = &bank_forks.read().unwrap().working_bank();
            PohRecorder::new_with_clear_signal(
                bank.tick_height(),
                bank.last_blockhash(),
                bank.clone(),
                None,
                bank.ticks_per_slot(),
                &id,
                &blockstore,
                blockstore.get_new_shred_signal(0),
                &leader_schedule_cache,
                &poh_config,
                Some(poh_timing_point_sender),
                exit.clone(),
            )
        };
        let poh_recorder = Arc::new(Mutex::new(poh_recorder));

        let connection_cache = Arc::new(ConnectionCache::new(use_quic, tpu_connection_pool_size));

        let rpc_override_health_check = Arc::new(AtomicBool::new(false));
        let (
            json_rpc_service,
            pubsub_service,
            optimistically_confirmed_bank_tracker,
            bank_notification_sender,
        ) = if let Some((rpc_addr, rpc_pubsub_addr)) = config.rpc_addrs {
            if ContactInfo::is_valid_address(&node.info.rpc, &socket_addr_space) {
                assert!(ContactInfo::is_valid_address(
                    &node.info.rpc_pubsub,
                    &socket_addr_space
                ));
            } else {
                assert!(!ContactInfo::is_valid_address(
                    &node.info.rpc_pubsub,
                    &socket_addr_space
                ));
            }

            let (bank_notification_sender, bank_notification_receiver) = unbounded();
            let confirmed_bank_subscribers = if !bank_notification_senders.is_empty() {
                Some(Arc::new(RwLock::new(bank_notification_senders)))
            } else {
                None
            };

            let json_rpc_service = JsonRpcService::new(
                rpc_addr,
                config.rpc_config.clone(),
                config.snapshot_config.clone(),
                bank_forks.clone(),
                block_commitment_cache.clone(),
                blockstore.clone(),
                cluster_info.clone(),
                Some(poh_recorder.clone()),
                genesis_config.hash(),
                ledger_path,
                config.validator_exit.clone(),
                config.known_validators.clone(),
                rpc_override_health_check.clone(),
                optimistically_confirmed_bank.clone(),
                config.send_transaction_service_config.clone(),
                max_slots.clone(),
                leader_schedule_cache.clone(),
                connection_cache.clone(),
                max_complete_transaction_status_slot,
            )
            .unwrap_or_else(|s| {
                error!("Failed to create JSON RPC Service: {}", s);
                abort();
            });

            (
                Some(json_rpc_service),
                if !config.rpc_config.full_api {
                    None
                } else {
                    let (trigger, pubsub_service) = PubSubService::new(
                        config.pubsub_config.clone(),
                        &rpc_subscriptions,
                        rpc_pubsub_addr,
                    );
                    config
                        .validator_exit
                        .write()
                        .unwrap()
                        .register_exit(Box::new(move || trigger.cancel()));

                    Some(pubsub_service)
                },
                Some(OptimisticallyConfirmedBankTracker::new(
                    bank_notification_receiver,
                    &exit,
                    bank_forks.clone(),
                    optimistically_confirmed_bank,
                    rpc_subscriptions.clone(),
                    confirmed_bank_subscribers,
                )),
                Some(bank_notification_sender),
            )
        } else {
            (None, None, None, None)
        };

        if config.halt_at_slot.is_some() {
            // Simulate a confirmed root to avoid RPC errors with CommitmentConfig::finalized() and
            // to ensure RPC endpoints like getConfirmedBlock, which require a confirmed root, work
            block_commitment_cache
                .write()
                .unwrap()
                .set_highest_confirmed_root(bank_forks.read().unwrap().root());

            // Park with the RPC service running, ready for inspection!
            warn!("Validator halted");
            *start_progress.write().unwrap() = ValidatorStartProgress::Halted;
            std::thread::park();
        }
        let ip_echo_server = match node.sockets.ip_echo {
            None => None,
            Some(tcp_listener) => Some(solana_net_utils::ip_echo_server(
                tcp_listener,
                Some(node.info.shred_version),
            )),
        };

        let (stats_reporter_sender, stats_reporter_receiver) = unbounded();

        let stats_reporter_service = StatsReporterService::new(stats_reporter_receiver, &exit);

        let gossip_service = GossipService::new(
            &cluster_info,
            Some(bank_forks.clone()),
            node.sockets.gossip,
            config.gossip_validators.clone(),
            should_check_duplicate_instance,
            Some(stats_reporter_sender.clone()),
            &exit,
        );
        let serve_repair = Arc::new(RwLock::new(ServeRepair::new(cluster_info.clone())));
        let serve_repair_service = ServeRepairService::new(
            &serve_repair,
            Some(blockstore.clone()),
            node.sockets.serve_repair,
            socket_addr_space,
            stats_reporter_sender,
            &exit,
        );

        let waited_for_supermajority = if let Ok(waited) = wait_for_supermajority(
            config,
            Some(&mut process_blockstore),
            &bank_forks,
            &cluster_info,
            rpc_override_health_check,
            &start_progress,
        ) {
            waited
        } else {
            abort();
        };

        let ledger_metric_report_service =
            LedgerMetricReportService::new(blockstore.clone(), &exit);

        let wait_for_vote_to_start_leader =
            !waited_for_supermajority && !config.no_wait_for_vote_to_start_leader;

        let poh_service = PohService::new(
            poh_recorder.clone(),
            &poh_config,
            &exit,
            bank_forks.read().unwrap().root_bank().ticks_per_slot(),
            config.poh_pinned_cpu_core,
            config.poh_hashes_per_batch,
            record_receiver,
        );
        assert_eq!(
            blockstore.get_new_shred_signals_len(),
            1,
            "New shred signal for the TVU should be the same as the clear bank signal."
        );

        let vote_tracker = Arc::<VoteTracker>::default();
        let mut cost_model = CostModel::default();
        // initialize cost model with built-in instruction costs only
        cost_model.initialize_cost_table(&[]);
        let cost_model = Arc::new(RwLock::new(cost_model));

        let (retransmit_slots_sender, retransmit_slots_receiver) = unbounded();
        let (verified_vote_sender, verified_vote_receiver) = unbounded();
        let (gossip_verified_vote_hash_sender, gossip_verified_vote_hash_receiver) = unbounded();
        let (cluster_confirmed_slot_sender, cluster_confirmed_slot_receiver) = unbounded();

        let rpc_completed_slots_service = RpcCompletedSlotsService::spawn(
            completed_slots_receiver,
            rpc_subscriptions.clone(),
            exit.clone(),
        );

        let (replay_vote_sender, replay_vote_receiver) = unbounded();
        let tvu = Tvu::new(
            vote_account,
            authorized_voter_keypairs,
            &bank_forks,
            &cluster_info,
            TvuSockets {
                repair: node.sockets.repair,
                retransmit: node.sockets.retransmit_sockets,
                fetch: node.sockets.tvu,
                forwards: node.sockets.tvu_forwards,
                ancestor_hashes_requests: node.sockets.ancestor_hashes_requests,
            },
            blockstore.clone(),
            ledger_signal_receiver,
            &rpc_subscriptions,
            &poh_recorder,
            process_blockstore,
            config.tower_storage.clone(),
            &leader_schedule_cache,
            &exit,
            block_commitment_cache,
            config.turbine_disabled.clone(),
            transaction_status_sender.clone(),
            rewards_recorder_sender,
            cache_block_meta_sender,
            vote_tracker.clone(),
            retransmit_slots_sender,
            gossip_verified_vote_hash_receiver,
            verified_vote_receiver,
            replay_vote_sender.clone(),
            completed_data_sets_sender,
            bank_notification_sender.clone(),
            cluster_confirmed_slot_receiver,
            TvuConfig {
                max_ledger_shreds: config.max_ledger_shreds,
                shred_version: node.info.shred_version,
                repair_validators: config.repair_validators.clone(),
                rocksdb_compaction_interval: config.rocksdb_compaction_interval,
                rocksdb_max_compaction_jitter: config.rocksdb_compaction_interval,
                wait_for_vote_to_start_leader,
            },
            &max_slots,
            &cost_model,
            block_metadata_notifier,
            config.wait_to_vote_slot,
            accounts_background_request_sender,
            &connection_cache,
        );

        let enable_quic_servers = if genesis_config.cluster_type == ClusterType::MainnetBeta {
            config.enable_quic_servers
        } else {
            if config.enable_quic_servers {
                warn!(
                    "ignoring --enable-quic-servers. QUIC is always enabled for cluster type: {:?}",
                    genesis_config.cluster_type
                );
            }
            true
        };

        let tpu = Tpu::new(
            &cluster_info,
            &poh_recorder,
            entry_receiver,
            retransmit_slots_receiver,
            TpuSockets {
                transactions: node.sockets.tpu,
                transaction_forwards: node.sockets.tpu_forwards,
                vote: node.sockets.tpu_vote,
                broadcast: node.sockets.broadcast,
                transactions_quic: node.sockets.tpu_quic,
                transactions_forwards_quic: node.sockets.tpu_forwards_quic,
            },
            &rpc_subscriptions,
            transaction_status_sender,
            &blockstore,
            &config.broadcast_stage_type,
            &exit,
            node.info.shred_version,
            vote_tracker,
            bank_forks.clone(),
            verified_vote_sender,
            gossip_verified_vote_hash_sender,
            replay_vote_receiver,
            replay_vote_sender,
            bank_notification_sender,
            config.tpu_coalesce_ms,
            cluster_confirmed_slot_sender,
            &cost_model,
            &connection_cache,
            &identity_keypair,
            enable_quic_servers,
        );

        datapoint_info!(
            "validator-new",
            ("id", id.to_string(), String),
            ("version", solana_version::version!(), String)
        );

        *start_progress.write().unwrap() = ValidatorStartProgress::Running;
        Self {
            stats_reporter_service,
            gossip_service,
            serve_repair_service,
            json_rpc_service,
            pubsub_service,
            rpc_completed_slots_service,
            optimistically_confirmed_bank_tracker,
            transaction_status_service,
            rewards_recorder_service,
            cache_block_meta_service,
            system_monitor_service,
            sample_performance_service,
            poh_timing_report_service,
            snapshot_packager_service,
            completed_data_sets_service,
            tpu,
            tvu,
            poh_service,
            poh_recorder,
            ip_echo_server,
            validator_exit: config.validator_exit.clone(),
            cluster_info,
            bank_forks,
            blockstore,
            geyser_plugin_service,
            ledger_metric_report_service,
            accounts_background_service,
            accounts_hash_verifier,
        }
    }
