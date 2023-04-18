    fn test_leader_to_leader_transition() {
        // Create the leader node information
        let bootstrap_leader_keypair = Keypair::new();
        let bootstrap_leader_node =
            Node::new_localhost_with_pubkey(bootstrap_leader_keypair.pubkey());
        let bootstrap_leader_info = bootstrap_leader_node.info.clone();

        // Make a mint and a genesis entries for leader ledger
        let num_ending_ticks = 1;
        let (_, bootstrap_leader_ledger_path, genesis_entries) = create_tmp_sample_ledger(
            "test_leader_to_leader_transition",
            10_000,
            num_ending_ticks,
            bootstrap_leader_keypair.pubkey(),
            500,
        );

        let initial_tick_height = genesis_entries
            .iter()
            .skip(2)
            .fold(0, |tick_count, entry| tick_count + entry.is_tick() as u64);

        // Create the common leader scheduling configuration
        let num_slots_per_epoch = 3;
        let leader_rotation_interval = 5;
        let seed_rotation_interval = num_slots_per_epoch * leader_rotation_interval;
        let active_window_length = 5;

        // Set the bootstrap height to be bigger than the initial tick height.
        // Once the leader hits the bootstrap height ticks, because there are no other
        // choices in the active set, this leader will remain the leader in the next
        // epoch. In the next epoch, check that the same leader knows to shut down and
        // restart as a leader again.
        let bootstrap_height = initial_tick_height + 1;
        let leader_scheduler_config = LeaderSchedulerConfig::new(
            Some(bootstrap_height as u64),
            Some(leader_rotation_interval),
            Some(seed_rotation_interval),
            Some(active_window_length),
        );

        let bootstrap_leader_keypair = Arc::new(bootstrap_leader_keypair);
        let signer = VoteSignerProxy::new(
            &bootstrap_leader_keypair,
            Box::new(LocalVoteSigner::default()),
        );
        // Start up the leader
        let mut bootstrap_leader = Fullnode::new(
            bootstrap_leader_node,
            &bootstrap_leader_ledger_path,
            bootstrap_leader_keypair,
            Some(Arc::new(signer)),
            Some(bootstrap_leader_info.gossip),
            false,
            LeaderScheduler::new(&leader_scheduler_config),
            None,
        );

        // Wait for the leader to transition, ticks should cause the leader to
        // reach the height for leader rotation
        match bootstrap_leader.handle_role_transition().unwrap() {
            Some(FullnodeReturnType::LeaderToValidatorRotation) => (),
            _ => {
                panic!("Expected a leader transition");
            }
        }

        match bootstrap_leader.node_role {
            Some(NodeRole::Leader(_)) => (),
            _ => {
                panic!("Expected bootstrap leader to be a leader");
            }
        }

        bootstrap_leader.close().unwrap();
    }
