    fn test_process_ledger_options_entry_callback() {
        let GenesisBlockInfo {
            genesis_block,
            mint_keypair,
            ..
        } = create_genesis_block(100);
        let (ledger_path, last_entry_hash) = create_new_tmp_ledger!(&genesis_block);
        let blocktree =
            Blocktree::open(&ledger_path).expect("Expected to successfully open database ledger");
        let blockhash = genesis_block.hash();
        let keypairs = [Keypair::new(), Keypair::new(), Keypair::new()];

        let tx = system_transaction::create_user_account(
            &mint_keypair,
            &keypairs[0].pubkey(),
            1,
            blockhash,
        );
        let entry_1 = next_entry(&last_entry_hash, 1, vec![tx]);

        let tx = system_transaction::create_user_account(
            &mint_keypair,
            &keypairs[1].pubkey(),
            1,
            blockhash,
        );
        let entry_2 = next_entry(&entry_1.hash, 1, vec![tx]);

        let mut entries = vec![entry_1, entry_2];
        entries.extend(create_ticks(genesis_block.ticks_per_slot, last_entry_hash));
        blocktree
            .write_entries(
                1,
                0,
                0,
                genesis_block.ticks_per_slot,
                None,
                true,
                &Arc::new(Keypair::new()),
                entries,
            )
            .unwrap();

        let callback_counter: Arc<RwLock<usize>> = Arc::default();
        let entry_callback = {
            let counter = callback_counter.clone();
            let pubkeys: Vec<Pubkey> = keypairs.iter().map(|k| k.pubkey()).collect();
            Arc::new(move |bank: &Bank| {
                let mut counter = counter.write().unwrap();
                assert_eq!(bank.get_balance(&pubkeys[*counter]), 1);
                assert_eq!(bank.get_balance(&pubkeys[*counter + 1]), 0);
                *counter += 1;
            })
        };

        let opts = ProcessOptions {
            override_num_threads: Some(1),
            entry_callback: Some(entry_callback),
            ..ProcessOptions::default()
        };
        process_blocktree(&genesis_block, &blocktree, None, opts).unwrap();
        assert_eq!(*callback_counter.write().unwrap(), 2);
    }
