    pub fn process_ledger<I>(&self, entries: I) -> Result<(u64, Vec<Entry>)>
    where
        I: IntoIterator<Item = Entry>,
    {
        let mut entries = entries.into_iter();

        // The first item in the ledger is required to be an entry with zero num_hashes,
        // which implies its id can be used as the ledger's seed.
        let entry0 = entries.next().expect("invalid ledger: empty");

        // The second item in the ledger is a special transaction where the to and from
        // fields are the same. That entry should be treated as a deposit, not a
        // transfer to oneself.
        let entry1 = entries
            .next()
            .expect("invalid ledger: need at least 2 entries");
        {
            let tx = &entry1.transactions[0];
            let deposit = if let Instruction::NewContract(contract) = &tx.instruction {
                contract.plan.final_payment()
            } else {
                None
            }.expect("invalid ledger, needs to start with a contract");

            self.apply_payment(&deposit, &mut self.balances.write().unwrap());
        }
        self.register_entry_id(&entry0.id);
        self.register_entry_id(&entry1.id);

        let mut entry_count = 2;
        let mut tail = Vec::with_capacity(WINDOW_SIZE as usize);
        let mut next = Vec::with_capacity(WINDOW_SIZE as usize);

        for block in &entries.into_iter().chunks(WINDOW_SIZE as usize) {
            tail = next;
            next = block.collect();
            entry_count += self.process_blocks(next.clone())?;
        }

        tail.append(&mut next);

        if tail.len() < WINDOW_SIZE as usize {
            tail.insert(0, entry1);
            if tail.len() < WINDOW_SIZE as usize {
                tail.insert(0, entry0);
            }
        }

        Ok((entry_count, tail))
    }
