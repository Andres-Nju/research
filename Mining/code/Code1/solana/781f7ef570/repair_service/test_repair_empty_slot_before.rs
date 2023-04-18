    pub fn test_repair_empty_slot() {
        let blocktree_path = get_tmp_ledger_path("test_repair_empty_slot");
        {
            let blocktree = Blocktree::open(&blocktree_path).unwrap();

            let mut blobs = make_tiny_test_entries(1).to_blobs();
            blobs[0].set_index(1);
            blobs[0].set_slot(2);

            let mut repair_info = RepairInfo::new();

            // Write this blob to slot 2, should chain to slot 1, which we haven't received
            // any blobs for
            blocktree.write_blobs(&blobs).unwrap();
            // Check that repair tries to patch the empty slot
            assert_eq!(
                RepairService::generate_repairs(&blocktree, 2, &mut repair_info).unwrap(),
                vec![RepairType::Blob(1, 0), RepairType::Blob(2, 0)]
            );
        }
        Blocktree::destroy(&blocktree_path).expect("Expected successful database destruction");
    }
