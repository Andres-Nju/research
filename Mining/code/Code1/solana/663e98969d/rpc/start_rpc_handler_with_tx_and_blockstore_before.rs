    fn start_rpc_handler_with_tx_and_blockstore(
        pubkey: &Pubkey,
        blockstore_roots: Vec<Slot>,
        default_timestamp: i64,
    ) -> RpcHandler {
        let (bank_forks, alice, leader_vote_keypair) = new_bank_forks();
        let bank = bank_forks.read().unwrap().working_bank();

        let commitment_slot0 = BlockCommitment::new([8; MAX_LOCKOUT_HISTORY]);
        let commitment_slot1 = BlockCommitment::new([9; MAX_LOCKOUT_HISTORY]);
        let mut block_commitment: HashMap<u64, BlockCommitment> = HashMap::new();
        block_commitment
            .entry(0)
            .or_insert(commitment_slot0.clone());
        block_commitment
            .entry(1)
            .or_insert(commitment_slot1.clone());
        let block_commitment_cache =
            Arc::new(RwLock::new(BlockCommitmentCache::new(block_commitment, 42)));
        let ledger_path = get_tmp_ledger_path!();
        let blockstore = Blockstore::open(&ledger_path).unwrap();
        let blockstore = Arc::new(blockstore);

        let keypair1 = Keypair::new();
        let keypair2 = Keypair::new();
        let keypair3 = Keypair::new();
        bank.transfer(4, &alice, &keypair2.pubkey()).unwrap();
        let confirmed_block_signatures = create_test_transactions_and_populate_blockstore(
            vec![&alice, &keypair1, &keypair2, &keypair3],
            0,
            bank.clone(),
            blockstore.clone(),
        );

        // Add timestamp vote to blockstore
        let vote = Vote {
            slots: vec![1],
            hash: Hash::default(),
            timestamp: Some(default_timestamp),
        };
        let vote_ix = vote_instruction::vote(
            &leader_vote_keypair.pubkey(),
            &leader_vote_keypair.pubkey(),
            vote,
        );
        let vote_tx = Transaction::new_signed_instructions(
            &[&leader_vote_keypair],
            vec![vote_ix],
            Hash::default(),
        );
        let shreds = entries_to_test_shreds(
            vec![next_entry_mut(&mut Hash::default(), 0, vec![vote_tx])],
            1,
            0,
            true,
            0,
        );
        blockstore.insert_shreds(shreds, None, false).unwrap();
        blockstore.set_roots(&[1]).unwrap();

        let mut roots = blockstore_roots.clone();
        if !roots.is_empty() {
            roots.retain(|&x| x > 1);
            let mut parent_bank = bank;
            for (i, root) in roots.iter().enumerate() {
                let new_bank =
                    Bank::new_from_parent(&parent_bank, parent_bank.collector_id(), *root);
                parent_bank = bank_forks.write().unwrap().insert(new_bank);
                parent_bank.squash();
                bank_forks.write().unwrap().set_root(*root, &None);
                let parent = if i > 0 { roots[i - 1] } else { 1 };
                fill_blockstore_slot_with_ticks(&blockstore, 5, *root, parent, Hash::default());
            }
            blockstore.set_roots(&roots).unwrap();
            let new_bank = Bank::new_from_parent(
                &parent_bank,
                parent_bank.collector_id(),
                roots.iter().max().unwrap() + 1,
            );
            bank_forks.write().unwrap().insert(new_bank);
        }

        let bank = bank_forks.read().unwrap().working_bank();

        let leader_pubkey = *bank.collector_id();
        let exit = Arc::new(AtomicBool::new(false));
        let validator_exit = create_validator_exit(&exit);

        let blockhash = bank.confirmed_last_blockhash().0;
        let tx = system_transaction::transfer(&alice, pubkey, 20, blockhash);
        bank.process_transaction(&tx).expect("process transaction");

        let tx = system_transaction::transfer(&alice, &alice.pubkey(), 20, blockhash);
        let _ = bank.process_transaction(&tx);

        let request_processor = Arc::new(RwLock::new(JsonRpcRequestProcessor::new(
            JsonRpcConfig::default(),
            bank_forks.clone(),
            block_commitment_cache.clone(),
            blockstore,
            StorageState::default(),
            validator_exit,
        )));
        let cluster_info = Arc::new(RwLock::new(ClusterInfo::new_with_invalid_keypair(
            ContactInfo::default(),
        )));

        cluster_info
            .write()
            .unwrap()
            .insert_info(ContactInfo::new_with_pubkey_socketaddr(
                &leader_pubkey,
                &socketaddr!("127.0.0.1:1234"),
            ));

        let mut io = MetaIoHandler::default();
        let rpc = RpcSolImpl;
        io.extend_with(rpc.to_delegate());
        let meta = Meta {
            request_processor,
            cluster_info,
            genesis_hash: Hash::default(),
        };
        RpcHandler {
            io,
            meta,
            bank,
            bank_forks,
            blockhash,
            alice,
            leader_pubkey,
            leader_vote_keypair,
            block_commitment_cache,
            confirmed_block_signatures,
        }
    }
