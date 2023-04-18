    fn sanitize_layout_and_length(&self) -> (bool, usize) {
        let mut offset = 0;

        // This discards allocated accounts immediately after check at each loop iteration.
        //
        // This code should not reuse AppendVec.accounts() method as the current form or
        // extend it to be reused here because it would allow attackers to accumulate
        // some measurable amount of memory needlessly.
        let mut num_accounts = 0;
        while let Some((account, next_offset)) = self.get_account(offset) {
            if !account.sanitize() {
                return (false, num_accounts);
            }
            offset = next_offset;
            num_accounts += 1;
        }
        let aligned_current_len = u64_align!(self.current_len.load(Ordering::Relaxed));

        (offset == aligned_current_len, num_accounts)
    }
