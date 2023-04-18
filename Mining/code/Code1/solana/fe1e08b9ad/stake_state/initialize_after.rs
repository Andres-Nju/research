    fn initialize(
        &self,
        authorized: &Authorized,
        lockup: &Lockup,
        rent: &Rent,
    ) -> Result<(), InstructionError>;
    fn authorize(
        &self,
        signers: &HashSet<Pubkey>,
        new_authority: &Pubkey,
        stake_authorize: StakeAuthorize,
    ) -> Result<(), InstructionError>;
    fn authorize_with_seed(
        &self,
        authority_base: &KeyedAccount,
        authority_seed: &str,
        authority_owner: &Pubkey,
        new_authority: &Pubkey,
        stake_authorize: StakeAuthorize,
    ) -> Result<(), InstructionError>;
    fn delegate(
        &self,
        vote_account: &KeyedAccount,
        clock: &Clock,
        stake_history: &StakeHistory,
        config: &Config,
        signers: &HashSet<Pubkey>,
    ) -> Result<(), InstructionError>;
    fn deactivate(&self, clock: &Clock, signers: &HashSet<Pubkey>) -> Result<(), InstructionError>;
    fn set_lockup(
        &self,
        lockup: &LockupArgs,
        signers: &HashSet<Pubkey>,
    ) -> Result<(), InstructionError>;
    fn split(
        &self,
        lamports: u64,
        split_stake: &KeyedAccount,
        signers: &HashSet<Pubkey>,
    ) -> Result<(), InstructionError>;
    fn merge(
        &self,
        source_stake: &KeyedAccount,
        clock: &Clock,
        stake_history: &StakeHistory,
        signers: &HashSet<Pubkey>,
    ) -> Result<(), InstructionError>;
    fn withdraw(
        &self,
        lamports: u64,
        to: &KeyedAccount,
        clock: &Clock,
        stake_history: &StakeHistory,
        withdraw_authority: &KeyedAccount,
        custodian: Option<&KeyedAccount>,
    ) -> Result<(), InstructionError>;
}

impl<'a> StakeAccount for KeyedAccount<'a> {
    fn initialize(
        &self,
        authorized: &Authorized,
        lockup: &Lockup,
        rent: &Rent,
    ) -> Result<(), InstructionError> {
        if let StakeState::Uninitialized = self.state()? {
            let rent_exempt_reserve = rent.minimum_balance(self.data_len()?);

            if rent_exempt_reserve < self.lamports()? {
                self.set_state(&StakeState::Initialized(Meta {
                    rent_exempt_reserve,
                    authorized: *authorized,
                    lockup: *lockup,
                }))
            } else {
                Err(InstructionError::InsufficientFunds)
            }
        } else {
            Err(InstructionError::InvalidAccountData)
        }
    }

    /// Authorize the given pubkey to manage stake (deactivate, withdraw). This may be called
    /// multiple times, but will implicitly withdraw authorization from the previously authorized
    /// staker. The default staker is the owner of the stake account's pubkey.
    fn authorize(
        &self,
        signers: &HashSet<Pubkey>,
        new_authority: &Pubkey,
        stake_authorize: StakeAuthorize,
    ) -> Result<(), InstructionError> {
        match self.state()? {
            StakeState::Stake(mut meta, stake) => {
                meta.authorized
                    .authorize(signers, new_authority, stake_authorize)?;
                self.set_state(&StakeState::Stake(meta, stake))
            }
            StakeState::Initialized(mut meta) => {
                meta.authorized
                    .authorize(signers, new_authority, stake_authorize)?;
                self.set_state(&StakeState::Initialized(meta))
            }
            _ => Err(InstructionError::InvalidAccountData),
        }
    }
    fn authorize_with_seed(
        &self,
        authority_base: &KeyedAccount,
        authority_seed: &str,
        authority_owner: &Pubkey,
        new_authority: &Pubkey,
        stake_authorize: StakeAuthorize,
    ) -> Result<(), InstructionError> {
        let mut signers = HashSet::default();
        if let Some(base_pubkey) = authority_base.signer_key() {
            signers.insert(Pubkey::create_with_seed(
                base_pubkey,
                authority_seed,
                authority_owner,
            )?);
        }
        self.authorize(&signers, &new_authority, stake_authorize)
    }
    fn delegate(
        &self,
        vote_account: &KeyedAccount,
        clock: &Clock,
        stake_history: &StakeHistory,
        config: &Config,
        signers: &HashSet<Pubkey>,
    ) -> Result<(), InstructionError> {
        match self.state()? {
            StakeState::Initialized(meta) => {
                meta.authorized.check(signers, StakeAuthorize::Staker)?;
                let stake = Stake::new(
                    self.lamports()?.saturating_sub(meta.rent_exempt_reserve), // can't stake the rent ;)
                    vote_account.unsigned_key(),
                    &State::<VoteStateVersions>::state(vote_account)?.convert_to_current(),
                    clock.epoch,
                    config,
                );
                self.set_state(&StakeState::Stake(meta, stake))
            }
            StakeState::Stake(meta, mut stake) => {
                meta.authorized.check(signers, StakeAuthorize::Staker)?;
                stake.redelegate(
                    self.lamports()?.saturating_sub(meta.rent_exempt_reserve), // can't stake the rent ;)
                    vote_account.unsigned_key(),
                    &State::<VoteStateVersions>::state(vote_account)?.convert_to_current(),
                    clock,
                    stake_history,
                    config,
                )?;
                self.set_state(&StakeState::Stake(meta, stake))
            }
            _ => Err(InstructionError::InvalidAccountData),
        }
    }
    fn deactivate(&self, clock: &Clock, signers: &HashSet<Pubkey>) -> Result<(), InstructionError> {
        if let StakeState::Stake(meta, mut stake) = self.state()? {
            meta.authorized.check(signers, StakeAuthorize::Staker)?;
            stake.deactivate(clock.epoch)?;

            self.set_state(&StakeState::Stake(meta, stake))
        } else {
            Err(InstructionError::InvalidAccountData)
        }
    }
    fn set_lockup(
        &self,
        lockup: &LockupArgs,
        signers: &HashSet<Pubkey>,
    ) -> Result<(), InstructionError> {
        match self.state()? {
            StakeState::Initialized(mut meta) => {
                meta.set_lockup(lockup, signers)?;
                self.set_state(&StakeState::Initialized(meta))
            }
            StakeState::Stake(mut meta, stake) => {
                meta.set_lockup(lockup, signers)?;
                self.set_state(&StakeState::Stake(meta, stake))
            }
            _ => Err(InstructionError::InvalidAccountData),
        }
    }

    fn split(
        &self,
        lamports: u64,
        split: &KeyedAccount,
        signers: &HashSet<Pubkey>,
    ) -> Result<(), InstructionError> {
        if let StakeState::Uninitialized = split.state()? {
            // verify enough account lamports
            if lamports > self.lamports()? {
                return Err(InstructionError::InsufficientFunds);
            }

            match self.state()? {
                StakeState::Stake(meta, mut stake) => {
                    meta.authorized.check(signers, StakeAuthorize::Staker)?;

                    // verify enough lamports for rent in new stake with the split
                    if split.lamports()? + lamports < meta.rent_exempt_reserve
                     // verify enough lamports left in previous stake and not full withdrawal
                        || (lamports + meta.rent_exempt_reserve > self.lamports()? && lamports != self.lamports()?)
                    {
                        return Err(InstructionError::InsufficientFunds);
                    }
                    // split the stake, subtract rent_exempt_balance unless
                    //  the destination account already has those lamports
                    //  in place.
                    // this could represent a small loss of staked lamports
                    //  if the split account starts out with a zero balance
                    let split_stake = stake.split(
                        lamports - meta.rent_exempt_reserve.saturating_sub(split.lamports()?),
                    )?;

                    self.set_state(&StakeState::Stake(meta, stake))?;
                    split.set_state(&StakeState::Stake(meta, split_stake))?;
                }
                StakeState::Initialized(meta) => {
                    meta.authorized.check(signers, StakeAuthorize::Staker)?;

                    // enough lamports for rent in new stake
                    if lamports < meta.rent_exempt_reserve
                    // verify enough lamports left in previous stake
                        || (lamports + meta.rent_exempt_reserve > self.lamports()? && lamports != self.lamports()?)
                    {
                        return Err(InstructionError::InsufficientFunds);
                    }

                    split.set_state(&StakeState::Initialized(meta))?;
                }
                StakeState::Uninitialized => {
                    if !signers.contains(&self.unsigned_key()) {
                        return Err(InstructionError::MissingRequiredSignature);
                    }
                }
                _ => return Err(InstructionError::InvalidAccountData),
            }

            split.try_account_ref_mut()?.lamports += lamports;
            self.try_account_ref_mut()?.lamports -= lamports;
            Ok(())
        } else {
            Err(InstructionError::InvalidAccountData)
        }
    }

    fn merge(
        &self,
        source_stake: &KeyedAccount,
        clock: &Clock,
        stake_history: &StakeHistory,
        signers: &HashSet<Pubkey>,
    ) -> Result<(), InstructionError> {
        let meta = match self.state()? {
            StakeState::Stake(meta, stake) => {
                // stake must be fully de-activated
                if stake.stake(clock.epoch, Some(stake_history)) != 0 {
                    return Err(StakeError::MergeActivatedStake.into());
                }
                meta
            }
            StakeState::Initialized(meta) => meta,
            _ => return Err(InstructionError::InvalidAccountData),
        };
        // Authorized staker is allowed to split/merge accounts
        meta.authorized.check(signers, StakeAuthorize::Staker)?;

        let source_meta = match source_stake.state()? {
            StakeState::Stake(meta, stake) => {
                // stake must be fully de-activated
                if stake.stake(clock.epoch, Some(stake_history)) != 0 {
                    return Err(StakeError::MergeActivatedStake.into());
                }
                meta
            }
            StakeState::Initialized(meta) => meta,
            _ => return Err(InstructionError::InvalidAccountData),
        };

        // Meta must match for both accounts
        if meta != source_meta {
            return Err(StakeError::MergeMismatch.into());
        }

        // Drain the source stake account
        let lamports = source_stake.lamports()?;
        source_stake.try_account_ref_mut()?.lamports -= lamports;
        self.try_account_ref_mut()?.lamports += lamports;
        Ok(())
    }

    fn withdraw(
        &self,
        lamports: u64,
        to: &KeyedAccount,
        clock: &Clock,
        stake_history: &StakeHistory,
        withdraw_authority: &KeyedAccount,
        custodian: Option<&KeyedAccount>,
    ) -> Result<(), InstructionError> {
        let mut signers = HashSet::new();
        let withdraw_authority_pubkey = withdraw_authority
            .signer_key()
            .ok_or(InstructionError::MissingRequiredSignature)?;
        signers.insert(*withdraw_authority_pubkey);

        let (lockup, reserve, is_staked) = match self.state()? {
            StakeState::Stake(meta, stake) => {
                meta.authorized
                    .check(&signers, StakeAuthorize::Withdrawer)?;
                // if we have a deactivation epoch and we're in cooldown
                let staked = if clock.epoch >= stake.delegation.deactivation_epoch {
                    stake.delegation.stake(clock.epoch, Some(stake_history))
                } else {
                    // Assume full stake if the stake account hasn't been
                    //  de-activated, because in the future the exposed stake
                    //  might be higher than stake.stake() due to warmup
                    stake.delegation.stake
                };

                (meta.lockup, staked + meta.rent_exempt_reserve, staked != 0)
            }
            StakeState::Initialized(meta) => {
                meta.authorized
                    .check(&signers, StakeAuthorize::Withdrawer)?;

                (meta.lockup, meta.rent_exempt_reserve, false)
            }
            StakeState::Uninitialized => {
                if !signers.contains(&self.unsigned_key()) {
                    return Err(InstructionError::MissingRequiredSignature);
                }
                (Lockup::default(), 0, false) // no lockup, no restrictions
            }
            _ => return Err(InstructionError::InvalidAccountData),
        };

        // verify that lockup has expired or that the withdrawal is signed by
        //   the custodian, both epoch and unix_timestamp must have passed
        let custodian_pubkey = custodian.and_then(|keyed_account| keyed_account.signer_key());
        if lockup.is_in_force(&clock, custodian_pubkey) {
            return Err(StakeError::LockupInForce.into());
        }

        // if the stake is active, we mustn't allow the account to go away
        if is_staked // line coverage for branch coverage
            && lamports + reserve > self.lamports()?
        {
            return Err(InstructionError::InsufficientFunds);
        }

        if lamports != self.lamports()? // not a full withdrawal
            && lamports + reserve > self.lamports()?
        {
            assert!(!is_staked);
            return Err(InstructionError::InsufficientFunds);
        }

        self.try_account_ref_mut()?.lamports -= lamports;
        to.try_account_ref_mut()?.lamports += lamports;
        Ok(())
    }
}
