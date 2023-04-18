    fn test_ensure_no_storage_rewards_pool() {
        solana_logger::setup();

        let mut genesis_config =
            create_genesis_config_with_leader(5, &Pubkey::new_rand(), 0).genesis_config;

        // OperatingMode::Preview - Storage rewards pool is purged at epoch 93
        // Also this is with bad capitalization
        genesis_config.operating_mode = OperatingMode::Preview;
        genesis_config.inflation = Inflation::default();
        let reward_pubkey = Pubkey::new_rand();
        genesis_config.rewards_pools.insert(
            reward_pubkey,
            Account::new(u64::MAX, 0, &Pubkey::new_rand()),
        );
        let bank0 = Bank::new(&genesis_config);
        // because capitalization has been reset with bogus capitalization calculation allowing overflows,
        // deliberately substract 1 lamport to simulate it
        bank0.capitalization.fetch_sub(1, Ordering::Relaxed);
        let bank0 = Arc::new(bank0);
        assert_eq!(bank0.get_balance(&reward_pubkey), u64::MAX,);

        let bank1 = Bank::new_from_parent(
            &bank0,
            &Pubkey::default(),
            genesis_config.epoch_schedule.get_first_slot_in_epoch(93),
        );

        // assert that everything gets in order....
        assert!(bank1.get_account(&reward_pubkey).is_none());
        assert_eq!(bank0.capitalization() + 1, bank1.capitalization());
        assert_eq!(bank1.capitalization(), bank1.calculate_capitalization());

        // Depending on RUSTFLAGS, this test exposes rust's checked math behavior or not...
        // So do some convolted setup; anyway this test itself will just be temporary
        let bank0 = std::panic::AssertUnwindSafe(bank0);
        let overflowing_capitalization =
            std::panic::catch_unwind(|| bank0.calculate_capitalization());
        if let Ok(overflowing_capitalization) = overflowing_capitalization {
            info!("asserting overflowing capitalization for bank0");
            assert_eq!(overflowing_capitalization, bank0.capitalization());
        } else {
            info!("NOT-asserting overflowing capitalization for bank0");
        }
    }
