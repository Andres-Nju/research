    fn test_replicator_startup() {
        logger::setup();
        info!("starting replicator test");
        let entry_height = 0;
        let replicator_ledger_path = "replicator_test_replicator_ledger";

        let exit = Arc::new(AtomicBool::new(false));

        let leader_ledger_path = "replicator_test_leader_ledger";
        let (mint, leader_ledger_path) = genesis(leader_ledger_path, 100);

        info!("starting leader node");
        let leader_keypair = Keypair::new();
        let leader_node = Node::new_localhost_with_pubkey(leader_keypair.pubkey());
        let network_addr = leader_node.sockets.gossip.local_addr().unwrap();
        let leader_info = leader_node.info.clone();
        let leader_rotation_interval = 20;
        let leader = Fullnode::new(
            leader_node,
            &leader_ledger_path,
            leader_keypair,
            None,
            false,
            Some(leader_rotation_interval),
        );

        let mut leader_client = mk_client(&leader_info);

        let bob = Keypair::new();

        let last_id = leader_client.get_last_id();
        leader_client
            .transfer(1, &mint.keypair(), bob.pubkey(), &last_id)
            .unwrap();

        let replicator_keypair = Keypair::new();

        info!("starting replicator node");
        let replicator_node = Node::new_localhost_with_pubkey(replicator_keypair.pubkey());
        let replicator = Replicator::new(
            entry_height,
            &exit,
            Some(replicator_ledger_path),
            replicator_node,
            Some(network_addr),
        );

        let mut num_entries = 0;
        for _ in 0..10 {
            match read_ledger(replicator_ledger_path, true) {
                Ok(entries) => {
                    for _ in entries {
                        num_entries += 1;
                    }
                    info!("{} entries", num_entries);
                    if num_entries > 0 {
                        break;
                    }
                }
                Err(e) => {
                    info!("error reading ledger: {:?}", e);
                }
            }
            sleep(Duration::new(1, 0));
            let last_id = leader_client.get_last_id();
            leader_client
                .transfer(1, &mint.keypair(), bob.pubkey(), &last_id)
                .unwrap();
        }
        assert!(num_entries > 0);
        exit.store(true, Ordering::Relaxed);
        replicator.join();
        leader.exit();
        let _ignored = remove_dir_all(&leader_ledger_path);
        let _ignored = remove_dir_all(&replicator_ledger_path);
    }
