fn test_multi_fec_block_different_size_coding() {
    let slot = 0x1234_5678_9abc_def0;
    let parent_slot = slot - 5;
    let keypair = Arc::new(Keypair::new());
    let (fec_data, fec_coding, num_shreds_per_iter) =
        setup_different_sized_fec_blocks(slot, parent_slot, keypair.clone());

    let total_num_data_shreds: usize = fec_data.values().map(|x| x.len()).sum();
    let reed_solomon_cache = ReedSolomonCache::default();
    // Test recovery
    for (fec_data_shreds, fec_coding_shreds) in fec_data.values().zip(fec_coding.values()) {
        let first_data_index = fec_data_shreds.first().unwrap().index() as usize;
        let all_shreds: Vec<Shred> = fec_data_shreds
            .iter()
            .step_by(2)
            .chain(fec_coding_shreds.iter().step_by(2))
            .cloned()
            .collect();
        let recovered_data = Shredder::try_recovery(all_shreds, &reed_solomon_cache).unwrap();
        // Necessary in order to ensure the last shred in the slot
        // is part of the recovered set, and that the below `index`
        // calcuation in the loop is correct
        assert!(fec_data_shreds.len() % 2 == 0);
        for (i, recovered_shred) in recovered_data.into_iter().enumerate() {
            let index = first_data_index + (i * 2) + 1;
            verify_test_data_shred(
                &recovered_shred,
                index.try_into().unwrap(),
                slot,
                parent_slot,
                &keypair.pubkey(),
                true,
                index == total_num_data_shreds - 1,
                index % num_shreds_per_iter == num_shreds_per_iter - 1,
            );
        }
    }
}
