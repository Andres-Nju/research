pub fn process_instruction(
    _program_id: &Pubkey,
    keyed_accounts: &mut [KeyedAccount],
    data: &[u8],
) -> Result<(), InstructionError> {
    solana_logger::setup_with_filter("solana=warn");

    trace!("process_instruction: {:?}", data);
    trace!("keyed_accounts: {:?}", keyed_accounts);

    if keyed_accounts.is_empty() {
        Err(InstructionError::InvalidInstructionData)?;
    }

    // 0th index is vote account
    let (me, rest) = &mut keyed_accounts.split_at_mut(1);
    let me = &mut me[0];

    // TODO: data-driven unpack and dispatch of KeyedAccounts
    match deserialize(data).map_err(|_| InstructionError::InvalidInstructionData)? {
        VoteInstruction::InitializeAccount(node_pubkey, commission) => {
            vote_state::initialize_account(me, &node_pubkey, commission)
        }
        VoteInstruction::AuthorizeVoter(voter_pubkey) => {
            vote_state::authorize_voter(me, rest, &voter_pubkey)
        }
        VoteInstruction::Vote(votes) => {
            datapoint_warn!("vote-native", ("count", 1, i64));
            if rest.len() < 2 {
                Err(InstructionError::InvalidInstructionData)?;
            }
            let (slot_hashes_and_clock, other_signers) = rest.split_at_mut(2);

            vote_state::process_votes(
                me,
                &sysvar::slot_hashes::from_keyed_account(&slot_hashes_and_clock[0])?,
                &sysvar::clock::from_keyed_account(&slot_hashes_and_clock[1])?,
                other_signers,
                &votes,
            )
        }
        VoteInstruction::Withdraw(lamports) => {
            if rest.is_empty() {
                Err(InstructionError::InvalidInstructionData)?;
            }
            vote_state::withdraw(me, lamports, &mut rest[0])
        }
    }
}
