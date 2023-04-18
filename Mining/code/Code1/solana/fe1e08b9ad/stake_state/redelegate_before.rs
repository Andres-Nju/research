    fn redelegate(
        &mut self,
        voter_pubkey: &Pubkey,
        vote_state: &VoteState,
        clock: &Clock,
        stake_history: &StakeHistory,
        config: &Config,
    ) -> Result<(), StakeError> {
        // can't redelegate if stake is active.  either the stake
        //  is freshly activated or has fully de-activated.  redelegation
        //  implies re-activation
        if self.stake(clock.epoch, Some(stake_history)) != 0 {
            return Err(StakeError::TooSoonToRedelegate);
        }
        self.delegation.activation_epoch = clock.epoch;
        self.delegation.deactivation_epoch = std::u64::MAX;
        self.delegation.voter_pubkey = *voter_pubkey;
        self.delegation.warmup_cooldown_rate = config.warmup_cooldown_rate;
        self.credits_observed = vote_state.credits();
        Ok(())
    }
