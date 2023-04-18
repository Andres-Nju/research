    fn test_accounts_data_size_and_resize_transactions() {
        let GenesisConfigInfo {
            genesis_config,
            mint_keypair,
            ..
        } = genesis_utils::create_genesis_config(100 * LAMPORTS_PER_SOL);
        let mut bank = Bank::new_for_tests(&genesis_config);
        let mock_program_id = Pubkey::new_unique();
        bank.add_builtin(
            "mock_realloc_program",
            &mock_program_id,
            mock_realloc_process_instruction,
        );
        let recent_blockhash = bank.last_blockhash();

        let funding_keypair = Keypair::new();
        bank.store_account(
            &funding_keypair.pubkey(),
            &AccountSharedData::new(10 * LAMPORTS_PER_SOL, 0, &mock_program_id),
        );

        let mut rng = rand::thread_rng();

        // Test case: Grow account
        {
            let account_pubkey = Pubkey::new_unique();
            let account_balance = LAMPORTS_PER_SOL;
            let account_size = rng.gen_range(
                1,
                MAX_PERMITTED_DATA_LENGTH as usize - MAX_PERMITTED_DATA_INCREASE,
            );
            let account_data =
                AccountSharedData::new(account_balance, account_size, &mock_program_id);
            bank.store_account(&account_pubkey, &account_data);

            let accounts_data_size_before = bank.load_accounts_data_size();
            let account_grow_size = rng.gen_range(1, MAX_PERMITTED_DATA_INCREASE);
            let transaction = create_mock_realloc_tx(
                &mint_keypair,
                &funding_keypair,
                &account_pubkey,
                account_size + account_grow_size,
                account_balance,
                mock_program_id,
                recent_blockhash,
            );
            let result = bank.process_transaction(&transaction);
            assert!(result.is_ok());
            let accounts_data_size_after = bank.load_accounts_data_size();
            assert_eq!(
                accounts_data_size_after,
                accounts_data_size_before.saturating_add(account_grow_size as u64),
            );
        }

        // Test case: Shrink account
        {
            let account_pubkey = Pubkey::new_unique();
            let account_balance = LAMPORTS_PER_SOL;
            let account_size =
                rng.gen_range(MAX_PERMITTED_DATA_LENGTH / 2, MAX_PERMITTED_DATA_LENGTH) as usize;
            let account_data =
                AccountSharedData::new(account_balance, account_size, &mock_program_id);
            bank.store_account(&account_pubkey, &account_data);

            let accounts_data_size_before = bank.load_accounts_data_size();
            let account_shrink_size = rng.gen_range(1, account_size);
            let transaction = create_mock_realloc_tx(
                &mint_keypair,
                &funding_keypair,
                &account_pubkey,
                account_size - account_shrink_size,
                account_balance,
                mock_program_id,
                recent_blockhash,
            );
            let result = bank.process_transaction(&transaction);
            assert!(result.is_ok());
            let accounts_data_size_after = bank.load_accounts_data_size();
            assert_eq!(
                accounts_data_size_after,
                accounts_data_size_before.saturating_sub(account_shrink_size as u64),
            );
        }
    }
