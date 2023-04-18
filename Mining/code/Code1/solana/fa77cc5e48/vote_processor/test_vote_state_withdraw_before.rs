    fn test_vote_state_withdraw() {
        let authorized_withdrawer_pubkey = solana_sdk::pubkey::new_rand();
        let (vote_pubkey_1, vote_account_with_epoch_credits_1) =
            create_test_account_with_epoch_credits(&[2, 1]);
        let (vote_pubkey_2, vote_account_with_epoch_credits_2) =
            create_test_account_with_epoch_credits(&[2, 1, 3]);
        let clock = Clock {
            epoch: 3,
            ..Clock::default()
        };
        let clock_account = account::create_account_shared_data_for_test(&clock);
        let rent_sysvar = Rent::default();
        let minimum_balance = rent_sysvar
            .minimum_balance(vote_account_with_epoch_credits_1.data().len())
            .max(1);
        let lamports = vote_account_with_epoch_credits_1.lamports();
        let transaction_accounts = vec![
            (vote_pubkey_1, vote_account_with_epoch_credits_1),
            (vote_pubkey_2, vote_account_with_epoch_credits_2),
            (sysvar::clock::id(), clock_account),
            (
                sysvar::rent::id(),
                account::create_account_shared_data_for_test(&rent_sysvar),
            ),
            (authorized_withdrawer_pubkey, AccountSharedData::default()),
        ];
        let mut instruction_accounts = vec![
            AccountMeta {
                pubkey: vote_pubkey_1,
                is_signer: true,
                is_writable: false,
            },
            AccountMeta {
                pubkey: sysvar::clock::id(),
                is_signer: false,
                is_writable: false,
            },
        ];

        // non rent exempt withdraw, with 0 credit epoch
        instruction_accounts[0].pubkey = vote_pubkey_1;
        process_instruction(
            &serialize(&VoteInstruction::Withdraw(lamports - minimum_balance + 1)).unwrap(),
            transaction_accounts.clone(),
            instruction_accounts.clone(),
            Err(InstructionError::InsufficientFunds),
        );

        // non rent exempt withdraw, without 0 credit epoch
        instruction_accounts[0].pubkey = vote_pubkey_2;
        process_instruction(
            &serialize(&VoteInstruction::Withdraw(lamports - minimum_balance + 1)).unwrap(),
            transaction_accounts.clone(),
            instruction_accounts.clone(),
            Err(InstructionError::InsufficientFunds),
        );

        // full withdraw, with 0 credit epoch
        instruction_accounts[0].pubkey = vote_pubkey_1;
        process_instruction(
            &serialize(&VoteInstruction::Withdraw(lamports)).unwrap(),
            transaction_accounts.clone(),
            instruction_accounts.clone(),
            Ok(()),
        );

        // full withdraw, without 0 credit epoch
        instruction_accounts[0].pubkey = vote_pubkey_2;
        process_instruction(
            &serialize(&VoteInstruction::Withdraw(lamports)).unwrap(),
            transaction_accounts.clone(),
            instruction_accounts.clone(),
            Err(InstructionError::ActiveVoteAccountClose),
        );

        // Both features disabled:
        // reject_non_rent_exempt_vote_withdraws
        // reject_vote_account_close_unless_zero_credit_epoch

        // non rent exempt withdraw, with 0 credit epoch
        instruction_accounts[0].pubkey = vote_pubkey_1;
        process_instruction_disabled_features(
            &serialize(&VoteInstruction::Withdraw(lamports - minimum_balance + 1)).unwrap(),
            transaction_accounts.clone(),
            instruction_accounts.clone(),
            Ok(()),
        );

        // non rent exempt withdraw, without 0 credit epoch
        instruction_accounts[0].pubkey = vote_pubkey_2;
        process_instruction_disabled_features(
            &serialize(&VoteInstruction::Withdraw(lamports - minimum_balance + 1)).unwrap(),
            transaction_accounts.clone(),
            instruction_accounts.clone(),
            Ok(()),
        );

        // full withdraw, with 0 credit epoch
        instruction_accounts[0].pubkey = vote_pubkey_1;
        process_instruction_disabled_features(
            &serialize(&VoteInstruction::Withdraw(lamports)).unwrap(),
            transaction_accounts.clone(),
            instruction_accounts.clone(),
            Ok(()),
        );

        // full withdraw, without 0 credit epoch
        instruction_accounts[0].pubkey = vote_pubkey_2;
        process_instruction_disabled_features(
            &serialize(&VoteInstruction::Withdraw(lamports)).unwrap(),
            transaction_accounts,
            instruction_accounts,
            Ok(()),
        );
    }
