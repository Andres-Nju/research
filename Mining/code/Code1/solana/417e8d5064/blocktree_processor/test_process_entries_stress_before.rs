    fn test_process_entries_stress() {
        // this test throws lots of rayon threads at process_entries()
        //  finds bugs in very low-layer stuff
        solana_logger::setup();
        let GenesisBlockInfo {
            genesis_block,
            mint_keypair,
            ..
        } = create_genesis_block(1_000_000_000);
        let mut bank = Bank::new(&genesis_block);

        const NUM_TRANSFERS: usize = 100;
        let keypairs: Vec<_> = (0..NUM_TRANSFERS * 2).map(|_| Keypair::new()).collect();

        // give everybody one lamport
        for keypair in &keypairs {
            bank.transfer(1, &mint_keypair, &keypair.pubkey())
                .expect("funding failed");
        }

        let mut i = 0;
        let mut hash = bank.last_blockhash();
        loop {
            let entries: Vec<_> = (0..NUM_TRANSFERS)
                .map(|i| {
                    next_entry_mut(
                        &mut hash,
                        0,
                        vec![system_transaction::transfer(
                            &keypairs[i],
                            &keypairs[i + NUM_TRANSFERS].pubkey(),
                            1,
                            bank.last_blockhash(),
                        )],
                    )
                })
                .collect();
            info!("paying iteration {}", i);
            process_entries(&bank, &entries).expect("paying failed");

            let entries: Vec<_> = (0..NUM_TRANSFERS)
                .map(|i| {
                    next_entry_mut(
                        &mut hash,
                        0,
                        vec![system_transaction::transfer(
                            &keypairs[i + NUM_TRANSFERS],
                            &keypairs[i].pubkey(),
                            1,
                            bank.last_blockhash(),
                        )],
                    )
                })
                .collect();

            info!("refunding iteration {}", i);
            process_entries(&bank, &entries).expect("refunding failed");

            // advance to next block
            process_entries(
                &bank,
                &(0..bank.ticks_per_slot())
                    .map(|_| next_entry_mut(&mut hash, 1, vec![]))
                    .collect::<Vec<_>>(),
            )
            .expect("process ticks failed");

            i += 1;
            bank = Bank::new_from_parent(&Arc::new(bank), &Pubkey::default(), i as u64);
            bank.squash();
        }
    }
