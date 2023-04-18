    pub(crate) fn delegation(&self) -> Delegation {
        // Safe to unwrap here because StakeAccount<Delegation> will always
        // only wrap a stake-state which is a delegation.
        self.stake_state.delegation().unwrap()
    }
