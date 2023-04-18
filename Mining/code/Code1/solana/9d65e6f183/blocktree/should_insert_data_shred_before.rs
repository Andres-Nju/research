    fn should_insert_data_shred(
        shred: &Shred,
        slot_meta: &SlotMeta,
        data_index: &DataIndex,
        last_root: &RwLock<u64>,
    ) -> bool {
        let shred_index = u64::from(shred.index());
        let slot = shred.slot();
        let last_in_slot = if shred.last_in_slot() {
            debug!("got last in slot");
            true
        } else {
            false
        };

        // Check that the data shred doesn't already exist in blocktree
        if shred_index < slot_meta.consumed || data_index.is_present(shred_index) {
            return false;
        }

        // Check that we do not receive shred_index >= than the last_index
        // for the slot
        let last_index = slot_meta.last_index;
        if shred_index >= last_index {
            datapoint_error!(
                "blocktree_error",
                (
                    "error",
                    format!(
                        "Received index {} >= slot.last_index {}",
                        shred_index, last_index
                    ),
                    String
                )
            );
            return false;
        }
        // Check that we do not receive a blob with "last_index" true, but shred_index
        // less than our current received
        if last_in_slot && shred_index < slot_meta.received {
            datapoint_error!(
                "blocktree_error",
                (
                    "error",
                    format!(
                        "Received shred_index {} < slot.received {}",
                        shred_index, slot_meta.received
                    ),
                    String
                )
            );
            return false;
        }

        let last_root = *last_root.read().unwrap();
        verify_shred_slots(slot, slot_meta.parent_slot, last_root);

        true
    }
