    pub fn update(&mut self, slot: Slot, account_info: T, reclaims: &mut SlotList<T>) {
        let mut addref = !account_info.is_cached();
        self.slot_list_mut(|list| {
            addref =
                InMemAccountsIndex::update_slot_list(list, slot, account_info, reclaims, false);
        });
        if addref {
            // If it's the first non-cache insert, also bump the stored ref count
            self.borrow_owned_entry().add_un_ref(true);
        }
    }
