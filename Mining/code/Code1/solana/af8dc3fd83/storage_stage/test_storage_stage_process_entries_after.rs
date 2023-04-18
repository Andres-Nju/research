    fn test_storage_stage_process_entries() {
        logger::setup();
        let keypair = Arc::new(Keypair::new());
        let exit = Arc::new(AtomicBool::new(false));

        let (_mint, ledger_path, _genesis) =
            create_tmp_sample_ledger("storage_stage_process_entries", 1000, 1);

        let entries = make_tiny_test_entries(128);
        {
            let mut writer = LedgerWriter::open(&ledger_path, true).unwrap();
            writer.write_entries(&entries.clone()).unwrap();
            // drops writer, flushes buffers
        }

        let (storage_entry_sender, storage_entry_receiver) = channel();
        let storage_state = StorageState::new();
        let storage_stage = StorageStage::new(
            &storage_state,
            storage_entry_receiver,
            Some(&ledger_path),
            keypair,
            exit.clone(),
            0,
        );
        storage_entry_sender.send(entries.clone()).unwrap();

        let keypair = Keypair::new();
        let mut result = storage_state.get_mining_result(&keypair.pubkey());
        assert_eq!(result, Hash::default());

        for _ in 0..9 {
            storage_entry_sender.send(entries.clone()).unwrap();
        }
        for _ in 0..5 {
            result = storage_state.get_mining_result(&keypair.pubkey());
            if result != Hash::default() {
                info!("found result = {:?} sleeping..", result);
                break;
            }
            info!("result = {:?} sleeping..", result);
            sleep(Duration::new(1, 0));
        }

        info!("joining..?");
        exit.store(true, Ordering::Relaxed);
        storage_stage.join().unwrap();

        #[cfg(not(all(feature = "cuda", feature = "chacha")))]
        assert_eq!(result, Hash::default());

        #[cfg(all(feature = "cuda", feature = "chacha"))]
        assert_ne!(result, Hash::default());

        remove_dir_all(ledger_path).unwrap();
    }
