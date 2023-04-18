    fn check_slots_are_valid(
        &self,
        vote: &Vote,
        slot_hashes: &[(Slot, Hash)],
    ) -> Result<(), VoteError> {
        let mut i = 0; // index into the vote's slots
        let mut j = slot_hashes.len(); // index into the slot_hashes
        while i < vote.slots.len() && j > 0 {
            // find the last slot in the vote
            if self
                .last_voted_slot()
                .map_or(false, |last_voted_slot| vote.slots[i] <= last_voted_slot)
            {
                i += 1;
                continue;
            }
            if vote.slots[i] != slot_hashes[j - 1].0 {
                j -= 1;
                continue;
            }
            i += 1;
            j -= 1;
        }
        if j == slot_hashes.len() {
            debug!(
                "{} dropped vote {:?} too old: {:?} ",
                self.node_pubkey, vote, slot_hashes
            );
            return Err(VoteError::VoteTooOld);
        }
        if i != vote.slots.len() {
            warn!(
                "{} dropped vote {:?} failed to match slot:  {:?}",
                self.node_pubkey, vote, slot_hashes,
            );
            inc_new_counter_info!("dropped-vote-slot", 1);
            return Err(VoteError::SlotsMismatch);
        }
        if slot_hashes[j].1 != vote.hash {
            warn!(
                "{} dropped vote {:?} failed to match hash {} {}",
                self.node_pubkey, vote, vote.hash, slot_hashes[j].1
            );
            inc_new_counter_info!("dropped-vote-hash", 1);
            return Err(VoteError::SlotHashMismatch);
        }
        Ok(())
    }
