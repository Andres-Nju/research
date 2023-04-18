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
                UPSERT_POPULATE_RECLAIMS,
            );
            assert!(gc.is_empty());
        }
