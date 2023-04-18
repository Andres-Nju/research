    pub fn test_insert_data_blobs_duplicate() {
        // Create RocksDb ledger
        let db_ledger_path = get_tmp_ledger_path("test_insert_data_blobs_duplicate");
        {
            let db_ledger = DbLedger::open(&db_ledger_path).unwrap();

            // Write entries
            let num_entries = 10 as u64;
            let num_duplicates = 2;
            let original_entries: Vec<Entry> = make_tiny_test_entries(num_entries as usize)
                .into_iter()
                .flat_map(|e| vec![e; num_duplicates])
                .collect();

            let shared_blobs = original_entries.clone().to_blobs();
            for (i, b) in shared_blobs.iter().enumerate() {
                let index = (i / 2) as u64;
                let mut w_b = b.write().unwrap();
                w_b.set_index(index).unwrap();
                w_b.set_slot(index).unwrap();
            }

            assert_eq!(
                db_ledger
                    .write_shared_blobs(
                        shared_blobs
                            .iter()
                            .skip(num_duplicates)
                            .step_by(num_duplicates * 2)
                    )
                    .unwrap(),
                vec![]
            );

            let expected: Vec<_> = original_entries
                .into_iter()
                .step_by(num_duplicates)
                .collect();

            assert_eq!(
                db_ledger
                    .write_shared_blobs(shared_blobs.iter().step_by(num_duplicates * 2))
                    .unwrap(),
                expected,
            );

            let meta_key = MetaCf::key(DEFAULT_SLOT_HEIGHT);
            let meta = db_ledger
                .meta_cf
                .get(&db_ledger.db, &meta_key)
                .unwrap()
                .unwrap();
            assert_eq!(meta.consumed, num_entries);
            assert_eq!(meta.received, num_entries);
            assert_eq!(meta.consumed_slot, num_entries - 1);
            assert_eq!(meta.received_slot, num_entries - 1);
        }
        DbLedger::destroy(&db_ledger_path).expect("Expected successful database destruction");
    }
