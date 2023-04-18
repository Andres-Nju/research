    fn new(
        pubkey: &Pubkey,
        loaded_account: &impl ReadableAccount,
        storage_slot: Slot,
        epoch_schedule: &EpochSchedule,
        rent_collector_max_epoch: &RentCollector,
        max_slot_in_storages_inclusive: &SlotInfoInEpoch,
        find_unskipped_slot: impl Fn(Slot) -> Option<Slot>,
        filler_account_suffix: Option<&Pubkey>,
    ) -> Option<Self> {
        let mut rent_collector = rent_collector_max_epoch;
        let SlotInfoInEpochInner {
            epoch: epoch_of_max_storage_slot,
            partition_index: partition_index_from_max_slot,
            slots_in_epoch: slots_per_epoch_max_epoch,
        } = max_slot_in_storages_inclusive.get_epoch_info(epoch_schedule);
        let mut partition_from_pubkey =
            crate::bank::Bank::partition_from_pubkey(pubkey, slots_per_epoch_max_epoch);
        // now, we have to find the root that is >= the slot where this pubkey's rent would have been collected
        let first_slot_in_max_epoch =
            max_slot_in_storages_inclusive.slot - partition_index_from_max_slot;
        let mut expected_rent_collection_slot_max_epoch =
            first_slot_in_max_epoch + partition_from_pubkey;
        let calculated_from_index_expected_rent_collection_slot_max_epoch =
            expected_rent_collection_slot_max_epoch;
        if expected_rent_collection_slot_max_epoch <= max_slot_in_storages_inclusive.slot {
            // may need to find a valid root
            if let Some(find) =
                find_unskipped_slot(calculated_from_index_expected_rent_collection_slot_max_epoch)
            {
                // found a root that is >= expected_rent_collection_slot.
                expected_rent_collection_slot_max_epoch = find;
            }
        }
        let mut use_previous_epoch_rent_collector = false;
        if expected_rent_collection_slot_max_epoch > max_slot_in_storages_inclusive.slot {
            // max slot has not hit the slot in the max epoch where we would have collected rent yet, so the most recent rent-collected rewrite slot for this pubkey would be in the previous epoch
            let previous_epoch = epoch_of_max_storage_slot.saturating_sub(1);
            let slots_per_epoch_previous_epoch = epoch_schedule.get_slots_in_epoch(previous_epoch);
            expected_rent_collection_slot_max_epoch =
                if slots_per_epoch_previous_epoch == slots_per_epoch_max_epoch {
                    // partition index remains the same
                    calculated_from_index_expected_rent_collection_slot_max_epoch
                        .saturating_sub(slots_per_epoch_max_epoch)
                } else {
                    // the newer epoch has a different # of slots, so the partition index will be different in the prior epoch
                    partition_from_pubkey = crate::bank::Bank::partition_from_pubkey(
                        pubkey,
                        slots_per_epoch_previous_epoch,
                    );
                    first_slot_in_max_epoch
                        .saturating_sub(slots_per_epoch_previous_epoch)
                        .saturating_add(partition_from_pubkey)
                };
            // since we are looking a different root, we have to call this again
            if let Some(find) = find_unskipped_slot(expected_rent_collection_slot_max_epoch) {
                // found a root (because we have a storage) that is >= expected_rent_collection_slot.
                expected_rent_collection_slot_max_epoch = find;
            }

            // since we have not hit the slot in the rent collector's epoch yet, we need to collect rent according to the previous epoch's rent collector.
            use_previous_epoch_rent_collector = true;
        }

        // the slot we're dealing with is where we expected the rent to be collected for this pubkey, so use what is in this slot
        // however, there are cases, such as adjusting the clock, where we store the account IN the same slot, but we do so BEFORE we collect rent. We later store the account AGAIN for rewrite/rent collection.
        // So, if storage_slot == expected_rent_collection_slot..., then we MAY have collected rent or may not have. So, it has to be >
        // rent_epoch=0 is a special case
        if storage_slot > expected_rent_collection_slot_max_epoch
            || loaded_account.rent_epoch() == 0
        {
            // no need to update hash
            return None;
        }

        let rent_collector_previous;
        if use_previous_epoch_rent_collector {
            // keep in mind the storage slot could be 0..inf epochs in the past
            // we want to swap the rent collector for one whose epoch is the previous epoch
            let mut rent_collector_temp = rent_collector.clone();
            rent_collector_temp.epoch = rent_collector.epoch.saturating_sub(1); // previous epoch
            rent_collector_previous = Some(rent_collector_temp);
            rent_collector = rent_collector_previous.as_ref().unwrap();
        }

        // ask the rent collector what rent should be collected.
        // Rent collector knows the current epoch.
        let rent_result = rent_collector.calculate_rent_result(
            pubkey,
            loaded_account,
            filler_account_suffix,
            // Skipping rewrites is not compatible with the below feature.
            // We will not skip rewrites until the feature is activated.
            false, // preserve_rent_epoch_for_rent_exempt_accounts
        );
        let current_rent_epoch = loaded_account.rent_epoch();
        let new_rent_epoch = match rent_result {
            RentResult::CollectRent {
                new_rent_epoch: next_epoch,
                rent_due,
            } => {
                if next_epoch > current_rent_epoch && rent_due != 0 {
                    // this is an account that would have had rent collected since this storage slot, so just use the hash we have since there must be a newer version of this account already in a newer slot
                    // It would be a waste of time to recalcluate a hash.
                    return None;
                }
                std::cmp::max(next_epoch, current_rent_epoch)
            }
            RentResult::LeaveAloneNoRent => {
                // rent_epoch is not updated for this condition
                // But, a rewrite WOULD HAVE occured at the expected slot.
                // So, fall through with same rent_epoch, but we will have already calculated 'expected_rent_collection_slot_max_epoch'
                current_rent_epoch
            }
        };

        if expected_rent_collection_slot_max_epoch == storage_slot
            && new_rent_epoch == loaded_account.rent_epoch()
        {
            // no rewrite would have occurred
            return None;
        }

        Some(Self {
            partition_from_pubkey,
            epoch_of_max_storage_slot,
            partition_index_from_max_slot,
            first_slot_in_max_epoch,
            expected_rent_collection_slot_max_epoch,
            rent_epoch: new_rent_epoch,
        })
    }
