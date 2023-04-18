    pub fn new<T: Into<Tower> + Sized>(
        config: ReplayStageConfig,
        blockstore: Arc<Blockstore>,
        bank_forks: Arc<RwLock<BankForks>>,
        cluster_info: Arc<ClusterInfo>,
        ledger_signal_receiver: Receiver<bool>,
        duplicate_slots_receiver: DuplicateSlotReceiver,
        poh_recorder: Arc<Mutex<PohRecorder>>,
        tower: T,
        vote_tracker: Arc<VoteTracker>,
        cluster_slots: Arc<ClusterSlots>,
        retransmit_slots_sender: RetransmitSlotsSender,
        epoch_slots_frozen_receiver: DuplicateSlotsResetReceiver,
        replay_vote_sender: ReplayVoteSender,
        gossip_duplicate_confirmed_slots_receiver: GossipDuplicateConfirmedSlotsReceiver,
        gossip_verified_vote_hash_receiver: GossipVerifiedVoteHashReceiver,
        cluster_slots_update_sender: ClusterSlotsUpdateSender,
        cost_update_sender: Sender<CostUpdate>,
        voting_sender: Sender<VoteOp>,
        drop_bank_sender: Sender<Vec<Arc<Bank>>>,
        block_metadata_notifier: Option<BlockMetadataNotifierLock>,
        transaction_cost_metrics_sender: Option<TransactionCostMetricsSender>,
    ) -> Self {
        let mut tower = tower.into();
        info!("Tower state: {:?}", tower);

        let ReplayStageConfig {
            vote_account,
            authorized_voter_keypairs,
            exit,
            rpc_subscriptions,
            leader_schedule_cache,
            latest_root_senders,
            accounts_background_request_sender,
            block_commitment_cache,
            transaction_status_sender,
            rewards_recorder_sender,
            cache_block_meta_sender,
            bank_notification_sender,
            wait_for_vote_to_start_leader,
            ancestor_hashes_replay_update_sender,
            tower_storage,
            wait_to_vote_slot,
        } = config;

        trace!("replay stage");
        // Start the replay stage loop
        let (lockouts_sender, commitment_service) = AggregateCommitmentService::new(
            &exit,
            block_commitment_cache.clone(),
            rpc_subscriptions.clone(),
        );

        #[allow(clippy::cognitive_complexity)]
        let t_replay = Builder::new()
            .name("solana-replay-stage".to_string())
            .spawn(move || {
                let verify_recyclers = VerifyRecyclers::default();
                let _exit = Finalizer::new(exit.clone());
                let mut identity_keypair = cluster_info.keypair().clone();
                let mut my_pubkey = identity_keypair.pubkey();
                let (
                    mut progress,
                    mut heaviest_subtree_fork_choice,
                ) = Self::initialize_progress_and_fork_choice_with_locked_bank_forks(
                    &bank_forks,
                    &my_pubkey,
                    &vote_account,
                );
                let mut current_leader = None;
                let mut last_reset = Hash::default();
                let mut partition_exists = false;
                let mut skipped_slots_info = SkippedSlotsInfo::default();
                let mut replay_timing = ReplayTiming::default();
                let mut duplicate_slots_tracker = DuplicateSlotsTracker::default();
                let mut gossip_duplicate_confirmed_slots: GossipDuplicateConfirmedSlots = GossipDuplicateConfirmedSlots::default();
                let mut epoch_slots_frozen_slots: EpochSlotsFrozenSlots = EpochSlotsFrozenSlots::default();
                let mut duplicate_slots_to_repair = DuplicateSlotsToRepair::default();
                let mut unfrozen_gossip_verified_vote_hashes: UnfrozenGossipVerifiedVoteHashes = UnfrozenGossipVerifiedVoteHashes::default();
                let mut latest_validator_votes_for_frozen_banks: LatestValidatorVotesForFrozenBanks = LatestValidatorVotesForFrozenBanks::default();
                let mut voted_signatures = Vec::new();
                let mut has_new_vote_been_rooted = !wait_for_vote_to_start_leader;
                let mut last_vote_refresh_time = LastVoteRefreshTime {
                    last_refresh_time: Instant::now(),
                    last_print_time: Instant::now(),
                };

                Self::reset_poh_recorder(
                    &my_pubkey,
                    &blockstore,
                    &bank_forks.read().unwrap().working_bank(),
                    &poh_recorder,
                    &leader_schedule_cache,
                );

                loop {
                    // Stop getting entries if we get exit signal
                    if exit.load(Ordering::Relaxed) {
                        break;
                    }

                    let mut generate_new_bank_forks_time =
                        Measure::start("generate_new_bank_forks_time");
                    Self::generate_new_bank_forks(
                        &blockstore,
                        &bank_forks,
                        &leader_schedule_cache,
                        &rpc_subscriptions,
                        &mut progress,
                        &mut replay_timing,
                    );
                    generate_new_bank_forks_time.stop();

                    let mut tpu_has_bank = poh_recorder.lock().unwrap().has_bank();

                    let mut replay_active_banks_time = Measure::start("replay_active_banks_time");
                    let mut ancestors = bank_forks.read().unwrap().ancestors();
                    let mut descendants = bank_forks.read().unwrap().descendants();
                    let did_complete_bank = Self::replay_active_banks(
                        &blockstore,
                        &bank_forks,
                        &my_pubkey,
                        &vote_account,
                        &mut progress,
                        transaction_status_sender.as_ref(),
                        cache_block_meta_sender.as_ref(),
                        &verify_recyclers,
                        &mut heaviest_subtree_fork_choice,
                        &replay_vote_sender,
                        &bank_notification_sender,
                        &rewards_recorder_sender,
                        &rpc_subscriptions,
                        &mut duplicate_slots_tracker,
                        &gossip_duplicate_confirmed_slots,
                        &mut epoch_slots_frozen_slots,
                        &mut unfrozen_gossip_verified_vote_hashes,
                        &mut latest_validator_votes_for_frozen_banks,
                        &cluster_slots_update_sender,
                        &cost_update_sender,
                        &mut duplicate_slots_to_repair,
                        &ancestor_hashes_replay_update_sender,
                        block_metadata_notifier.clone(),
                        transaction_cost_metrics_sender.as_ref(),
                    );
                    replay_active_banks_time.stop();

                    let forks_root = bank_forks.read().unwrap().root();

                    // Reset any dead slots that have been frozen by a sufficient portion of
                    // the network. Signalled by repair_service.
                    let mut purge_dead_slots_time = Measure::start("purge_dead_slots");
                    Self::process_epoch_slots_frozen_dead_slots(
                        &my_pubkey,
                        &blockstore,
                        &epoch_slots_frozen_receiver,
                        &mut duplicate_slots_tracker,
                        &gossip_duplicate_confirmed_slots,
                        &mut epoch_slots_frozen_slots,
                        &mut progress,
                        &mut heaviest_subtree_fork_choice,
                        &bank_forks,
                        &mut duplicate_slots_to_repair,
                        &ancestor_hashes_replay_update_sender
                    );
                    purge_dead_slots_time.stop();

                    // Check for any newly confirmed slots detected from gossip.
                    let mut process_gossip_duplicate_confirmed_slots_time = Measure::start("process_gossip_duplicate_confirmed_slots");
                    Self::process_gossip_duplicate_confirmed_slots(
                        &gossip_duplicate_confirmed_slots_receiver,
                        &blockstore,
                        &mut duplicate_slots_tracker,
                        &mut gossip_duplicate_confirmed_slots,
                        &mut epoch_slots_frozen_slots,
                        &bank_forks,
                        &mut progress,
                        &mut heaviest_subtree_fork_choice,
                        &mut duplicate_slots_to_repair,
                        &ancestor_hashes_replay_update_sender,
                    );
                    process_gossip_duplicate_confirmed_slots_time.stop();


                    // Ingest any new verified votes from gossip. Important for fork choice
                    // and switching proofs because these may be votes that haven't yet been
                    // included in a block, so we may not have yet observed these votes just
                    // by replaying blocks.
                    let mut process_unfrozen_gossip_verified_vote_hashes_time = Measure::start("process_gossip_duplicate_confirmed_slots");
                    Self::process_gossip_verified_vote_hashes(
                        &gossip_verified_vote_hash_receiver,
                        &mut unfrozen_gossip_verified_vote_hashes,
                        &heaviest_subtree_fork_choice,
                        &mut latest_validator_votes_for_frozen_banks,
                    );
                    for _ in gossip_verified_vote_hash_receiver.try_iter() {}
                    process_unfrozen_gossip_verified_vote_hashes_time.stop();

                    // Check to remove any duplicated slots from fork choice
                    let mut process_duplicate_slots_time = Measure::start("process_duplicate_slots");
                    if !tpu_has_bank {
                        Self::process_duplicate_slots(
                            &blockstore,
                            &duplicate_slots_receiver,
                            &mut duplicate_slots_tracker,
                            &gossip_duplicate_confirmed_slots,
                            &mut epoch_slots_frozen_slots,
                            &bank_forks,
                            &mut progress,
                            &mut heaviest_subtree_fork_choice,
                            &mut duplicate_slots_to_repair,
                            &ancestor_hashes_replay_update_sender,
                        );
                    }
                    process_duplicate_slots_time.stop();

                    let mut collect_frozen_banks_time = Measure::start("frozen_banks");
                    let mut frozen_banks: Vec<_> = bank_forks
                        .read()
                        .unwrap()
                        .frozen_banks()
                        .into_iter()
                        .filter(|(slot, _)| *slot >= forks_root)
                        .map(|(_, bank)| bank)
                        .collect();
                    collect_frozen_banks_time.stop();

                    let mut compute_bank_stats_time = Measure::start("compute_bank_stats");
                    let newly_computed_slot_stats = Self::compute_bank_stats(
                        &vote_account,
                        &ancestors,
                        &mut frozen_banks,
                        &mut tower,
                        &mut progress,
                        &vote_tracker,
                        &cluster_slots,
                        &bank_forks,
                        &mut heaviest_subtree_fork_choice,
                        &mut latest_validator_votes_for_frozen_banks,
                    );
                    compute_bank_stats_time.stop();

                    let mut compute_slot_stats_time = Measure::start("compute_slot_stats_time");
                    for slot in newly_computed_slot_stats {
                        let fork_stats = progress.get_fork_stats(slot).unwrap();
                        let confirmed_forks = Self::confirm_forks(
                            &tower,
                            &fork_stats.voted_stakes,
                            fork_stats.total_stake,
                            &progress,
                            &bank_forks,
                        );

                        Self::mark_slots_confirmed(&confirmed_forks, &blockstore, &bank_forks, &mut progress, &mut duplicate_slots_tracker, &mut heaviest_subtree_fork_choice,  &mut epoch_slots_frozen_slots, &mut duplicate_slots_to_repair, &ancestor_hashes_replay_update_sender);
                    }
                    compute_slot_stats_time.stop();

                    let mut select_forks_time = Measure::start("select_forks_time");
                    let (heaviest_bank, heaviest_bank_on_same_voted_fork) = heaviest_subtree_fork_choice
                        .select_forks(&frozen_banks, &tower, &progress, &ancestors, &bank_forks);
                    select_forks_time.stop();

                    if let Some(heaviest_bank_on_same_voted_fork) = heaviest_bank_on_same_voted_fork.as_ref() {
                        if let Some(my_latest_landed_vote) = progress.my_latest_landed_vote(heaviest_bank_on_same_voted_fork.slot()) {
                            Self::refresh_last_vote(&mut tower,
                                                    heaviest_bank_on_same_voted_fork,
                                                    my_latest_landed_vote,
                                                    &vote_account,
                                                    &identity_keypair,
                                                    &authorized_voter_keypairs.read().unwrap(),
                                                    &mut voted_signatures,
                                                    has_new_vote_been_rooted, &mut
                                                    last_vote_refresh_time,
                                                    &voting_sender,
                                                    wait_to_vote_slot,
                                                    );
                        }
                    }

                    let mut select_vote_and_reset_forks_time =
                        Measure::start("select_vote_and_reset_forks");
                    let SelectVoteAndResetForkResult {
                        vote_bank,
                        reset_bank,
                        heaviest_fork_failures,
                    } = Self::select_vote_and_reset_forks(
                        &heaviest_bank,
                        heaviest_bank_on_same_voted_fork.as_ref(),
                        &ancestors,
                        &descendants,
                        &progress,
                        &mut tower,
                        &latest_validator_votes_for_frozen_banks,
                        &heaviest_subtree_fork_choice,
                    );
                    select_vote_and_reset_forks_time.stop();

                    let mut heaviest_fork_failures_time = Measure::start("heaviest_fork_failures_time");
                    if tower.is_recent(heaviest_bank.slot()) && !heaviest_fork_failures.is_empty() {
                        info!(
                            "Couldn't vote on heaviest fork: {:?}, heaviest_fork_failures: {:?}",
                            heaviest_bank.slot(),
                            heaviest_fork_failures
                        );

                        for r in heaviest_fork_failures {
                            if let HeaviestForkFailures::NoPropagatedConfirmation(slot) = r {
                                if let Some(latest_leader_slot) =
                                    progress.get_latest_leader_slot_must_exist(slot)
                                {
                                    progress.log_propagated_stats(latest_leader_slot, &bank_forks);
                                }
                            }
                        }
                    }
                    heaviest_fork_failures_time.stop();

                    let mut voting_time = Measure::start("voting_time");
                    // Vote on a fork
                    if let Some((ref vote_bank, ref switch_fork_decision)) = vote_bank {
                        if let Some(votable_leader) =
                            leader_schedule_cache.slot_leader_at(vote_bank.slot(), Some(vote_bank))
                        {
                            Self::log_leader_change(
                                &my_pubkey,
                                vote_bank.slot(),
                                &mut current_leader,
                                &votable_leader,
                            );
                        }

                        Self::handle_votable_bank(
                            vote_bank,
                            switch_fork_decision,
                            &bank_forks,
                            &mut tower,
                            &mut progress,
                            &vote_account,
                            &identity_keypair,
                            &authorized_voter_keypairs.read().unwrap(),
                            &blockstore,
                            &leader_schedule_cache,
                            &lockouts_sender,
                            &accounts_background_request_sender,
                            &latest_root_senders,
                            &rpc_subscriptions,
                            &block_commitment_cache,
                            &mut heaviest_subtree_fork_choice,
                            &bank_notification_sender,
                            &mut duplicate_slots_tracker,
                            &mut gossip_duplicate_confirmed_slots,
                            &mut unfrozen_gossip_verified_vote_hashes,
                            &mut voted_signatures,
                            &mut has_new_vote_been_rooted,
                            &mut replay_timing,
                            &voting_sender,
                            &mut epoch_slots_frozen_slots,
                            &drop_bank_sender,
                            wait_to_vote_slot,
                        );
                    };
                    voting_time.stop();

                    let mut reset_bank_time = Measure::start("reset_bank");
                    // Reset onto a fork
                    if let Some(reset_bank) = reset_bank {
                        if last_reset != reset_bank.last_blockhash() {
                            info!(
                                "vote bank: {:?} reset bank: {:?}",
                                vote_bank.as_ref().map(|(b, switch_fork_decision)| (
                                    b.slot(),
                                    switch_fork_decision
                                )),
                                reset_bank.slot(),
                            );
                            let fork_progress = progress
                                .get(&reset_bank.slot())
                                .expect("bank to reset to must exist in progress map");
                            datapoint_info!(
                                "blocks_produced",
                                ("num_blocks_on_fork", fork_progress.num_blocks_on_fork, i64),
                                (
                                    "num_dropped_blocks_on_fork",
                                    fork_progress.num_dropped_blocks_on_fork,
                                    i64
                                ),
                            );

                            if my_pubkey != cluster_info.id() {
                                identity_keypair = cluster_info.keypair().clone();
                                let my_old_pubkey = my_pubkey;
                                my_pubkey = identity_keypair.pubkey();

                                // Load the new identity's tower
                                tower = Tower::restore(tower_storage.as_ref(), &my_pubkey)
                                    .and_then(|restored_tower| {
                                        let root_bank = bank_forks.read().unwrap().root_bank();
                                        let slot_history = root_bank.get_slot_history();
                                        restored_tower.adjust_lockouts_after_replay(root_bank.slot(), &slot_history)
                                    }).
                                    unwrap_or_else(|err| {
                                        if err.is_file_missing() {
                                            Tower::new_from_bankforks(
                                                &bank_forks.read().unwrap(),
                                                &my_pubkey,
                                                &vote_account,
                                            )
                                        } else {
                                            error!("Failed to load tower for {}: {}", my_pubkey, err);
                                            std::process::exit(1);
                                        }
                                    });

                                // Ensure the validator can land votes with the new identity before
                                // becoming leader
                                has_new_vote_been_rooted = !wait_for_vote_to_start_leader;
                                warn!("Identity changed from {} to {}", my_old_pubkey, my_pubkey);
                            }

                            Self::reset_poh_recorder(
                                &my_pubkey,
                                &blockstore,
                                &reset_bank,
                                &poh_recorder,
                                &leader_schedule_cache,
                            );
                            last_reset = reset_bank.last_blockhash();
                            tpu_has_bank = false;

                            if let Some(last_voted_slot) = tower.last_voted_slot() {
                                // If the current heaviest bank is not a descendant of the last voted slot,
                                // there must be a partition
                                let partition_detected = Self::is_partition_detected(&ancestors, last_voted_slot, heaviest_bank.slot());

                                if !partition_exists && partition_detected
                                {
                                    warn!(
                                        "PARTITION DETECTED waiting to join heaviest fork: {} last vote: {:?}, reset slot: {}",
                                        heaviest_bank.slot(),
                                        last_voted_slot,
                                        reset_bank.slot(),
                                    );
                                    inc_new_counter_info!("replay_stage-partition_detected", 1);
                                    datapoint_info!(
                                        "replay_stage-partition",
                                        ("slot", reset_bank.slot() as i64, i64)
                                    );
                                    partition_exists = true;
                                } else if partition_exists
                                    && !partition_detected
                                {
                                    warn!(
                                        "PARTITION resolved heaviest fork: {} last vote: {:?}, reset slot: {}",
                                        heaviest_bank.slot(),
                                        last_voted_slot,
                                        reset_bank.slot()
                                    );
                                    partition_exists = false;
                                    inc_new_counter_info!("replay_stage-partition_resolved", 1);
                                }
                            }
                        }
                    }
                    reset_bank_time.stop();

                    let mut start_leader_time = Measure::start("start_leader_time");
                    let mut dump_then_repair_correct_slots_time = Measure::start("dump_then_repair_correct_slots_time");
                    // Used for correctness check
                    let poh_bank = poh_recorder.lock().unwrap().bank();
                    // Dump any duplicate slots that have been confirmed by the network in
                    // anticipation of repairing the confirmed version of the slot.
                    //
                    // Has to be before `maybe_start_leader()`. Otherwise, `ancestors` and `descendants`
                    // will be outdated, and we cannot assume `poh_bank` will be in either of these maps.
                    Self::dump_then_repair_correct_slots(&mut duplicate_slots_to_repair, &mut ancestors, &mut descendants, &mut progress, &bank_forks, &blockstore, poh_bank.map(|bank| bank.slot()));
                    dump_then_repair_correct_slots_time.stop();

                    let mut retransmit_not_propagated_time = Measure::start("retransmit_not_propagated_time");
                    Self::retransmit_latest_unpropagated_leader_slot(
                        &poh_recorder,
                        &retransmit_slots_sender,
                        &mut progress,
                    );
                    retransmit_not_propagated_time.stop();

                    // From this point on, its not safe to use ancestors/descendants since maybe_start_leader
                    // may add a bank that will not included in either of these maps.
                    drop(ancestors);
                    drop(descendants);
                    if !tpu_has_bank {
                        Self::maybe_start_leader(
                            &my_pubkey,
                            &bank_forks,
                            &poh_recorder,
                            &leader_schedule_cache,
                            &rpc_subscriptions,
                            &mut progress,
                            &retransmit_slots_sender,
                            &mut skipped_slots_info,
                            has_new_vote_been_rooted,
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
                    start_leader_time.stop();

                    let mut wait_receive_time = Measure::start("wait_receive_time");
                    if !did_complete_bank {
                        // only wait for the signal if we did not just process a bank; maybe there are more slots available

                        let timer = Duration::from_millis(100);
                        let result = ledger_signal_receiver.recv_timeout(timer);
                        match result {
                            Err(RecvTimeoutError::Timeout) => (),
                            Err(_) => break,
                            Ok(_) => trace!("blockstore signal"),
                        };
                    }
                    wait_receive_time.stop();

                    replay_timing.update(
                        collect_frozen_banks_time.as_us(),
                        compute_bank_stats_time.as_us(),
                        select_vote_and_reset_forks_time.as_us(),
                        start_leader_time.as_us(),
                        reset_bank_time.as_us(),
                        voting_time.as_us(),
                        select_forks_time.as_us(),
                        compute_slot_stats_time.as_us(),
                        generate_new_bank_forks_time.as_us(),
                        replay_active_banks_time.as_us(),
                        wait_receive_time.as_us(),
                        heaviest_fork_failures_time.as_us(),
                        if did_complete_bank {1} else {0},
                        process_gossip_duplicate_confirmed_slots_time.as_us(),
                        process_unfrozen_gossip_verified_vote_hashes_time.as_us(),
                        process_duplicate_slots_time.as_us(),
                        dump_then_repair_correct_slots_time.as_us(),
                        retransmit_not_propagated_time.as_us(),
                    );
                }
            })
            .unwrap();

        Self {
            t_replay,
            commitment_service,
        }
    }
