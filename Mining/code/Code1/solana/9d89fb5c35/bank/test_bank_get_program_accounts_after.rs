    fn test_bank_get_program_accounts() {
        let (genesis_config, mint_keypair) = create_genesis_config(500);
        let parent = Arc::new(Bank::new(&genesis_config));
        parent.lazy_rent_collection.store(true, Ordering::Relaxed);

        let genesis_accounts: Vec<_> = parent.get_program_accounts(None);
        assert!(
            genesis_accounts
                .iter()
                .any(|(pubkey, _)| *pubkey == mint_keypair.pubkey()),
            "mint pubkey not found"
        );
        assert!(
            genesis_accounts
                .iter()
                .any(|(pubkey, _)| solana_sdk::sysvar::is_sysvar_id(pubkey)),
            "no sysvars found"
        );

        let bank0 = Arc::new(new_from_parent(&parent));
        let pubkey0 = Pubkey::new_rand();
        let program_id = Pubkey::new(&[2; 32]);
        let account0 = Account::new(1, 0, &program_id);
        bank0.store_account(&pubkey0, &account0);

        assert_eq!(
            bank0.get_program_accounts_modified_since_parent(&program_id),
            vec![(pubkey0, account0.clone())]
        );

        let bank1 = Arc::new(new_from_parent(&bank0));
        bank1.squash();
        assert_eq!(
            bank0.get_program_accounts(Some(&program_id)),
            vec![(pubkey0, account0.clone())]
        );
        assert_eq!(
            bank1.get_program_accounts(Some(&program_id)),
            vec![(pubkey0, account0)]
        );
        assert_eq!(
            bank1.get_program_accounts_modified_since_parent(&program_id),
            vec![]
        );

        let bank2 = Arc::new(new_from_parent(&bank1));
        let pubkey1 = Pubkey::new_rand();
        let account1 = Account::new(3, 0, &program_id);
        bank2.store_account(&pubkey1, &account1);
        // Accounts with 0 lamports should be filtered out by Accounts::load_by_program()
        let pubkey2 = Pubkey::new_rand();
        let account2 = Account::new(0, 0, &program_id);
        bank2.store_account(&pubkey2, &account2);

        let bank3 = Arc::new(new_from_parent(&bank2));
        bank3.squash();
        assert_eq!(bank1.get_program_accounts(Some(&program_id)).len(), 2);
        assert_eq!(bank3.get_program_accounts(Some(&program_id)).len(), 2);
    }
