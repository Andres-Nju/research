    fn test_get_corrected_rent_epoch_on_load() {
        solana_logger::setup();
        let pubkey = Pubkey::new(&[5; 32]);
        let owner = solana_sdk::pubkey::new_rand();
        let mut account = AccountSharedData::new(1, 0, &owner);
        let mut epoch_schedule = EpochSchedule {
            first_normal_epoch: 0,
            ..EpochSchedule::default()
        };
        epoch_schedule.first_normal_slot = 0;
        let first_normal_slot = epoch_schedule.first_normal_slot;
        let slots_per_epoch = 432_000;
        let partition_from_pubkey = 8470; // function of 432k slots and 'pubkey' above
                                          // start in epoch=1 because of issues at rent_epoch=1
        let storage_slot = first_normal_slot + partition_from_pubkey + slots_per_epoch;
        let epoch = epoch_schedule.get_epoch(storage_slot);
        assert_eq!(
            (epoch, partition_from_pubkey),
            epoch_schedule.get_epoch_and_slot_index(storage_slot)
        );
        let genesis_config = GenesisConfig::default();
        let mut rent_collector = RentCollector::new(
            epoch,
            epoch_schedule,
            genesis_config.slots_per_year(),
            genesis_config.rent,
        );
        rent_collector.rent.lamports_per_byte_year = 0; // temporarily disable rent

        assert_eq!(
            slots_per_epoch,
            epoch_schedule.get_slots_in_epoch(epoch_schedule.get_epoch(storage_slot))
        );
        account.set_rent_epoch(1); // has to be not 0

        /*
        test this:
        pubkey_partition_index: 8470
        storage_slot: 8470
        account.rent_epoch: 1 (has to be not 0)

        max_slot: 8469 + 432k * 1
        max_slot: 8470 + 432k * 1
        max_slot: 8471 + 432k * 1
        max_slot: 8472 + 432k * 1
        max_slot: 8469 + 432k * 2
        max_slot: 8470 + 432k * 2
        max_slot: 8471 + 432k * 2
        max_slot: 8472 + 432k * 2
        max_slot: 8469 + 432k * 3
        max_slot: 8470 + 432k * 3
        max_slot: 8471 + 432k * 3
        max_slot: 8472 + 432k * 3

        one run without skipping slot 8470, once WITH skipping slot 8470
        */

        for new_small in [false, true] {
            for rewrite_already in [false, true] {
                // starting at epoch = 0 has issues because of rent_epoch=0 special casing
                for epoch in 1..4 {
                    for partition_index_bank_slot in
                        partition_from_pubkey - 1..=partition_from_pubkey + 2
                    {
                        let bank_slot =
                            slots_per_epoch * epoch + first_normal_slot + partition_index_bank_slot;
                        if storage_slot > bank_slot {
                            continue; // illegal combination
                        }
                        rent_collector.epoch = epoch_schedule.get_epoch(bank_slot);
                        let first_slot_in_max_epoch = bank_slot - bank_slot % slots_per_epoch;

                        assert_eq!(
                            (epoch, partition_index_bank_slot),
                            epoch_schedule.get_epoch_and_slot_index(bank_slot)
                        );
                        assert_eq!(
                            (epoch, 0),
                            epoch_schedule.get_epoch_and_slot_index(first_slot_in_max_epoch)
                        );
                        account.set_rent_epoch(1);
                        let rewrites = Rewrites::default();
                        if rewrite_already {
                            if partition_index_bank_slot != partition_from_pubkey {
                                // this is an invalid test occurrence.
                                // we wouldn't have inserted pubkey into 'rewrite_already' for this slot if the current partition index wasn't at the pubkey's partition idnex yet.
                                continue;
                            }

                            rewrites.write().unwrap().insert(pubkey, Hash::default());
                        }
                        let expected_new_rent_epoch =
                            if partition_index_bank_slot > partition_from_pubkey {
                                if epoch > account.rent_epoch() {
                                    Some(rent_collector.epoch)
                                } else {
                                    None
                                }
                            } else if partition_index_bank_slot == partition_from_pubkey
                                && rewrite_already
                            {
                                let expected_rent_epoch = rent_collector.epoch;
                                if expected_rent_epoch == account.rent_epoch() {
                                    None
                                } else {
                                    Some(expected_rent_epoch)
                                }
                            } else if partition_index_bank_slot <= partition_from_pubkey
                                && epoch > account.rent_epoch()
                            {
                                let expected_rent_epoch = rent_collector.epoch.saturating_sub(1);
                                if expected_rent_epoch == account.rent_epoch() {
                                    None
                                } else {
                                    Some(expected_rent_epoch)
                                }
                            } else {
                                None
                            };
                        let get_slot_info = |slot| {
                            if new_small {
                                SlotInfoInEpoch::new_small(slot)
                            } else {
                                SlotInfoInEpoch::new(slot, &epoch_schedule)
                            }
                        };
                        let new_rent_epoch =
                            ExpectedRentCollection::get_corrected_rent_epoch_on_load(
                                &account,
                                &get_slot_info(storage_slot),
                                &get_slot_info(bank_slot),
                                &epoch_schedule,
                                &rent_collector,
                                &pubkey,
                                &rewrites,
                            );
                        assert_eq!(new_rent_epoch, expected_new_rent_epoch);
                    }
                }
            }
        }
    }
