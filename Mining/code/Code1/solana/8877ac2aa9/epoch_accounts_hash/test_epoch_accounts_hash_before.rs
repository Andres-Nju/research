fn test_epoch_accounts_hash() {
    solana_logger::setup();

    const NUM_EPOCHS_TO_TEST: u64 = 2;
    const SET_ROOT_INTERVAL: Slot = 3;

    let test_config = TestEnvironment::new();
    let bank_forks = &test_config.bank_forks;

    let mut expected_epoch_accounts_hash = None;

    let slots_per_epoch = test_config
        .genesis_config_info
        .genesis_config
        .epoch_schedule
        .slots_per_epoch;
    for _ in 0..slots_per_epoch * NUM_EPOCHS_TO_TEST {
        let bank = {
            let parent = bank_forks.read().unwrap().working_bank();
            let bank = bank_forks.write().unwrap().insert(Bank::new_from_parent(
                &parent,
                &Pubkey::default(),
                parent.slot() + 1,
            ));

            let transaction = system_transaction::transfer(
                &test_config.genesis_config_info.mint_keypair,
                &Pubkey::new_unique(),
                1,
                bank.last_blockhash(),
            );
            bank.process_transaction(&transaction).unwrap();
            bank.fill_bank_with_ticks_for_tests();

            bank
        };
        trace!("new bank {}", bank.slot());

        // Set roots so that ABS requests are sent (this is what requests EAH calculations)
        if bank.slot() % SET_ROOT_INTERVAL == 0 {
            trace!("rooting bank {}", bank.slot());
            bank_forks.write().unwrap().set_root(
                bank.slot(),
                &test_config
                    .background_services
                    .accounts_background_request_sender,
                None,
            );
        }

        // To ensure EAH calculations are correct, calculate the accounts hash here, in-band.
        // This will be the expected EAH that gets saved into the "stop" bank.
        if bank.slot() == epoch_accounts_hash::calculation_start(&bank) {
            bank.freeze();
            let (accounts_hash, _) = bank
                .rc
                .accounts
                .accounts_db
                .calculate_accounts_hash(
                    bank.slot(),
                    &CalcAccountsHashConfig {
                        use_bg_thread_pool: false,
                        check_hash: false,
                        ancestors: Some(&bank.ancestors),
                        use_write_cache: true,
                        epoch_schedule: bank.epoch_schedule(),
                        rent_collector: bank.rent_collector(),
                        store_detailed_debug_info_on_failure: false,
                        full_snapshot: None,
                        enable_rehashing: true,
                    },
                )
                .unwrap();
            expected_epoch_accounts_hash = Some(EpochAccountsHash::new(accounts_hash));
            debug!(
                "slot {}, expected epoch accounts hash: {:?}",
                bank.slot(),
                expected_epoch_accounts_hash
            );
        }

        // Test: Ensure that the "stop" bank has the correct EAH
        if bank.slot() == epoch_accounts_hash::calculation_stop(&bank) {
            // Sometimes AHV does not get scheduled to run, which causes the test to fail
            // spuriously.  Sleep a bit here to ensure AHV gets a chance to run.
            std::thread::sleep(Duration::from_secs(1));
            let actual_epoch_accounts_hash = bank.epoch_accounts_hash();
            debug!(
                "slot {},   actual epoch accounts hash: {:?}",
                bank.slot(),
                actual_epoch_accounts_hash,
            );
            assert_eq!(expected_epoch_accounts_hash, actual_epoch_accounts_hash);
        }

        // Give the background services a chance to run
        std::thread::yield_now();
    }
}
