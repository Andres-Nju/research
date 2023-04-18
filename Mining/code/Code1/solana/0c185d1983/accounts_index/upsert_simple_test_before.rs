        fn upsert_simple_test(&self, key: &Pubkey, slot: Slot, value: T) {
            let mut gc = Vec::new();
            self.upsert(
                slot,
                slot,
                key,
                &AccountSharedData::default(),
                &AccountSecondaryIndexes::default(),
                value,
                &mut gc,
                UPSERT_PREVIOUS_SLOT_ENTRY_WAS_CACHED_FALSE,
            );
            assert!(gc.is_empty());
        }
