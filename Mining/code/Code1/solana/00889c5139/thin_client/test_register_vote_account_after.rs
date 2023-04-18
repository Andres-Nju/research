    fn test_register_vote_account() {
        logger::setup();
        let leader_keypair = Arc::new(Keypair::new());
        let leader = Node::new_localhost_with_pubkey(leader_keypair.pubkey());
        let mint = Mint::new(10_000);
        let mut bank = Bank::new(&mint);
        let leader_data = leader.info.clone();
        let ledger_path = create_tmp_ledger_with_mint("client_check_signature", &mint);

        let genesis_entries = &mint.create_entries();
        let entry_height = genesis_entries.len() as u64;

        let leader_scheduler = Arc::new(RwLock::new(LeaderScheduler::from_bootstrap_leader(
            leader_data.id,
        )));
        bank.leader_scheduler = leader_scheduler;
        let leader_vote_account_keypair = Arc::new(Keypair::new());
        let server = Fullnode::new_with_bank(
            leader_keypair,
            leader_vote_account_keypair.clone(),
            bank,
            entry_height,
            &genesis_entries.last().unwrap().id,
            leader,
            None,
            &ledger_path,
            false,
            Some(0),
        );
        sleep(Duration::from_millis(300));

        let requests_socket = UdpSocket::bind("0.0.0.0:0").unwrap();
        requests_socket
            .set_read_timeout(Some(Duration::new(5, 0)))
            .unwrap();
        let transactions_socket = UdpSocket::bind("0.0.0.0:0").unwrap();
        let mut client = ThinClient::new(
            leader_data.contact_info.rpu,
            requests_socket,
            leader_data.contact_info.tpu,
            transactions_socket,
        );

        // Create the validator account, transfer some tokens to that account
        let validator_keypair = Keypair::new();
        let last_id = client.get_last_id();
        let signature = client
            .transfer(500, &mint.keypair(), validator_keypair.pubkey(), &last_id)
            .unwrap();

        assert!(client.poll_for_signature(&signature).is_ok());

        // Create the vote account
        let validator_vote_account_keypair = Keypair::new();
        let vote_account_id = validator_vote_account_keypair.pubkey();
        let last_id = client.get_last_id();
        let signature = client
            .create_vote_account(&validator_keypair, vote_account_id, &last_id, 1)
            .unwrap();

        assert!(client.poll_for_signature(&signature).is_ok());
        let balance = retry_get_balance(&mut client, &vote_account_id, Some(1))
            .expect("Expected balance for new account to exist");
        assert_eq!(balance, 1);

        // Register the vote account to the validator
        let last_id = client.get_last_id();
        let signature = client
            .register_vote_account(&validator_keypair, vote_account_id, &last_id)
            .unwrap();
        assert!(client.poll_for_signature(&signature).is_ok());

        const LAST: usize = 30;
        for run in 0..=LAST {
            println!("Checking for account registered: {}", run);
            let account_user_data = client
                .get_account_userdata(&vote_account_id)
                .expect("Expected valid response for account userdata")
                .expect("Expected valid account userdata to exist after account creation");

            let vote_state = VoteProgram::deserialize(&account_user_data);

            if vote_state.map(|vote_state| vote_state.node_id) == Ok(validator_keypair.pubkey()) {
                break;
            }

            if run == LAST {
                panic!("Expected successful vote account registration");
            }
            sleep(Duration::from_millis(900));
        }

        server.close().unwrap();
        remove_dir_all(ledger_path).unwrap();
    }
