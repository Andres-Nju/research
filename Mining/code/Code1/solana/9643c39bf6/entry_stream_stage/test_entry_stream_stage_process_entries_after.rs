    fn test_entry_stream_stage_process_entries() {
        // Set up bank and leader_scheduler
        let ticks_per_slot = 5;
        let leader_scheduler_config = LeaderSchedulerConfig::new(ticks_per_slot, 2, 10);
        let (genesis_block, _mint_keypair) = GenesisBlock::new(1_000_000);
        let bank = Bank::new_with_leader_scheduler_config(&genesis_block, &leader_scheduler_config);
        // Set up entry stream
        let mut entry_stream =
            EntryStream::new("test_stream".to_string(), bank.leader_scheduler.clone());

        // Set up dummy channels to host an EntryStreamStage
        let (ledger_entry_sender, ledger_entry_receiver) = channel();
        let (entry_stream_sender, entry_stream_receiver) = channel();

        let mut last_id = Hash::default();
        let mut entries = Vec::new();
        let mut expected_entries = Vec::new();
        for x in 0..6 {
            let entry = Entry::new(&mut last_id, x, 1, vec![]); //just ticks
            last_id = entry.id;
            expected_entries.push(entry.clone());
            entries.push(entry);
        }
        let keypair = Keypair::new();
        let tx = SystemTransaction::new_account(&keypair, keypair.pubkey(), 1, Hash::default(), 0);
        let entry = Entry::new(&mut last_id, ticks_per_slot - 1, 1, vec![tx]);
        expected_entries.insert(ticks_per_slot as usize, entry.clone());
        entries.insert(ticks_per_slot as usize, entry);

        ledger_entry_sender.send(entries).unwrap();
        EntryStreamStage::process_entries(
            &ledger_entry_receiver,
            &entry_stream_sender,
            &mut entry_stream,
        )
        .unwrap();
        assert_eq!(entry_stream.entries().len(), 8);

        let (entry_events, block_events): (Vec<Value>, Vec<Value>) = entry_stream
            .entries()
            .iter()
            .map(|item| {
                let json: Value = serde_json::from_str(&item).unwrap();
                let dt_str = json["dt"].as_str().unwrap();
                // Ensure `ts` field parses as valid DateTime
                let _dt: DateTime<FixedOffset> = DateTime::parse_from_rfc3339(dt_str).unwrap();
                json
            })
            .partition(|json| {
                let item_type = json["t"].as_str().unwrap();
                item_type == "entry"
            });
        for (i, json) in entry_events.iter().enumerate() {
            let entry_obj = json["entry"].clone();
            let tx = entry_obj["transactions"].as_array().unwrap();
            if tx.len() == 0 {
                // TODO: There is a bug in Transaction deserialize methods such that
                // `serde_json::from_str` does not work for populated Entries.
                // Remove this `if` when fixed.
                let entry: Entry = serde_json::from_value(entry_obj).unwrap();
                assert_eq!(entry, expected_entries[i]);
            }
        }
        for json in block_events {
            let slot = json["s"].as_u64().unwrap();
            assert_eq!(0, slot);
            let height = json["h"].as_u64().unwrap();
            assert_eq!(ticks_per_slot - 1, height);
        }

        // Ensure entries pass through stage unadulterated
        let recv_entries = entry_stream_receiver.recv().unwrap();
        assert_eq!(expected_entries, recv_entries);
    }
