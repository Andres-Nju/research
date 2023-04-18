pub fn withdraw<S: std::hash::BuildHasher>(
    transaction_context: &TransactionContext,
    instruction_context: &InstructionContext,
    vote_account_index: usize,
    lamports: u64,
    to_account_index: usize,
    signers: &HashSet<Pubkey, S>,
    rent_sysvar: Option<&Rent>,
    clock: Option<&Clock>,
) -> Result<(), InstructionError> {
    let mut vote_account = instruction_context
        .try_borrow_instruction_account(transaction_context, vote_account_index)?;
    let vote_state: VoteState = vote_account
        .get_state::<VoteStateVersions>()?
        .convert_to_current();

    verify_authorized_signer(&vote_state.authorized_withdrawer, signers)?;

    let remaining_balance = vote_account
        .get_lamports()
        .checked_sub(lamports)
        .ok_or(InstructionError::InsufficientFunds)?;

    if remaining_balance == 0 {
        let reject_active_vote_account_close = clock
            .zip(vote_state.epoch_credits.last())
            .map(|(clock, (last_epoch_with_credits, _, _))| {
                let current_epoch = clock.epoch;
                // if current_epoch - last_epoch_with_credits < 2 then the validator has received credits
                // either in the current epoch or the previous epoch. If it's >= 2 then it has been at least
                // one full epoch since the validator has received credits.
                current_epoch.saturating_sub(*last_epoch_with_credits) < 2
            })
            .unwrap_or(false);

        if reject_active_vote_account_close {
            datapoint_debug!("vote-account-close", ("reject-active", 1, i64));
            return Err(InstructionError::ActiveVoteAccountClose);
        } else {
            // Deinitialize upon zero-balance
            datapoint_debug!("vote-account-close", ("allow", 1, i64));
            vote_account.set_state(&VoteStateVersions::new_current(VoteState::default()))?;
        }
    } else if let Some(rent_sysvar) = rent_sysvar {
        let min_rent_exempt_balance = rent_sysvar.minimum_balance(vote_account.get_data().len());
        if remaining_balance < min_rent_exempt_balance {
            return Err(InstructionError::InsufficientFunds);
        }
    }

    vote_account.checked_sub_lamports(lamports)?;
    drop(vote_account);
    let mut to_account = instruction_context
        .try_borrow_instruction_account(transaction_context, to_account_index)?;
    to_account.checked_add_lamports(lamports)?;
    Ok(())
}
