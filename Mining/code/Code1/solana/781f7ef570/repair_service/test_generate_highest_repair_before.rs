    pub fn test_generate_highest_repair() {
        let blocktree_path = get_tmp_ledger_path("test_generate_repairs");
        {
            let blocktree = Blocktree::open(&blocktree_path).unwrap();

            let num_entries_per_slot = 10;

            let mut repair_info = RepairInfo::new();

            // Create some blobs
            let (mut blobs, _) = make_slot_entries(0, 0, num_entries_per_slot as u64);

            // Remove is_last flag on last blob
            blobs.last_mut().unwrap().set_flags(0);

            blocktree.write_blobs(&blobs).unwrap();

            // We didn't get the last blob for the slot, so ask for the highest blob for that slot
            let expected: Vec<RepairType> = vec![RepairType::HighestBlob(0, num_entries_per_slot)];

            assert_eq!(
                RepairService::generate_repairs(&blocktree, std::usize::MAX, &mut repair_info)
                    .unwrap(),
                expected
            );
        }
        Blocktree::destroy(&blocktree_path).expect("Expected successful database destruction");
    }
