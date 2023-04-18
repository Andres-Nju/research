    fn test_set_lockup() {
        let stake_pubkey = solana_sdk::pubkey::new_rand();
        let stake_lamports = 42;
        let stake_account = AccountSharedData::new_ref_data_with_space(
            stake_lamports,
            &StakeState::Uninitialized,
            std::mem::size_of::<StakeState>(),
            &id(),
        )
        .expect("stake_account");

        // wrong state, should fail
        let stake_keyed_account = KeyedAccount::new(&stake_pubkey, false, &stake_account);
        assert_eq!(
            stake_keyed_account.set_lockup(&LockupArgs::default(), &HashSet::default(), None),
            Err(InstructionError::InvalidAccountData)
        );

        // initialize the stake
        let custodian = solana_sdk::pubkey::new_rand();
        stake_keyed_account
            .initialize(
                &Authorized::auto(&stake_pubkey),
                &Lockup {
                    unix_timestamp: 1,
                    epoch: 1,
                    custodian,
                },
                &Rent::free(),
            )
            .unwrap();

        assert_eq!(
            stake_keyed_account.set_lockup(&LockupArgs::default(), &HashSet::default(), None),
            Err(InstructionError::MissingRequiredSignature)
        );

        assert_eq!(
            stake_keyed_account.set_lockup(
                &LockupArgs {
                    unix_timestamp: Some(1),
                    epoch: Some(1),
                    custodian: Some(custodian),
                },
                &vec![custodian].into_iter().collect(),
                None
            ),
            Ok(())
        );

        // delegate stake
        let vote_pubkey = solana_sdk::pubkey::new_rand();
        let vote_account = RefCell::new(vote_state::create_account(
            &vote_pubkey,
            &solana_sdk::pubkey::new_rand(),
            0,
            100,
        ));
        let vote_keyed_account = KeyedAccount::new(&vote_pubkey, false, &vote_account);
        vote_keyed_account
            .set_state(&VoteStateVersions::new_current(VoteState::default()))
            .unwrap();

        stake_keyed_account
            .delegate(
                &vote_keyed_account,
                &Clock::default(),
                &StakeHistory::default(),
                &Config::default(),
                &vec![stake_pubkey].into_iter().collect(),
                true,
            )
            .unwrap();

        assert_eq!(
            stake_keyed_account.set_lockup(
                &LockupArgs {
                    unix_timestamp: Some(1),
                    epoch: Some(1),
                    custodian: Some(custodian),
                },
                &HashSet::default(),
                None
            ),
            Err(InstructionError::MissingRequiredSignature)
        );
        assert_eq!(
            stake_keyed_account.set_lockup(
                &LockupArgs {
                    unix_timestamp: Some(1),
                    epoch: Some(1),
                    custodian: Some(custodian),
                },
                &vec![custodian].into_iter().collect(),
                None
            ),
            Ok(())
        );
    }
