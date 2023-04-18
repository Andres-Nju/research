    fn test_bank_update_rewards() {
        // create a bank that ticks really slowly...
        let bank = Arc::new(Bank::new(&GenesisConfig {
            accounts: (0..42)
                .into_iter()
                .map(|_| {
                    (
                        Pubkey::new_rand(),
                        Account::new(1_000_000_000, 0, &Pubkey::default()),
                    )
                })
                .collect(),
            // set it up so the first epoch is a full year long
            poh_config: PohConfig {
                target_tick_duration: Duration::from_secs(
                    SECONDS_PER_YEAR as u64
                        / MINIMUM_SLOTS_PER_EPOCH as u64
                        / DEFAULT_TICKS_PER_SLOT,
                ),
                hashes_per_tick: None,
                target_tick_count: None,
            },

            ..GenesisConfig::default()
        }));
        assert_eq!(bank.capitalization(), 42 * 1_000_000_000);
        assert_eq!(bank.rewards, None);

        let ((vote_id, mut vote_account), (stake_id, stake_account)) =
            crate::stakes::tests::create_staked_node_accounts(1_0000);

        let ((validator_id, validator_account), (archiver_id, archiver_account)) =
            crate::storage_utils::tests::create_storage_accounts_with_credits(100);

        // set up stakes, vote, and storage accounts
        bank.store_account(&stake_id, &stake_account);
        bank.store_account(&validator_id, &validator_account);
        bank.store_account(&archiver_id, &archiver_account);

        // generate some rewards
        let mut vote_state = Some(VoteState::from(&vote_account).unwrap());
        for i in 0..MAX_LOCKOUT_HISTORY + 42 {
            vote_state
                .as_mut()
                .map(|v| v.process_slot_vote_unchecked(i as u64));
            let versioned = VoteStateVersions::Current(Box::new(vote_state.take().unwrap()));
            VoteState::to(&versioned, &mut vote_account).unwrap();
            bank.store_account(&vote_id, &vote_account);
            match versioned {
                VoteStateVersions::Current(v) => {
                    vote_state = Some(*v);
                }
                _ => panic!("Has to be of type Current"),
            };
        }
        bank.store_account(&vote_id, &vote_account);

        let validator_points = bank.stakes.read().unwrap().points();
        let storage_points = bank.storage_accounts.read().unwrap().points();

        // put a child bank in epoch 1, which calls update_rewards()...
        let bank1 = Bank::new_from_parent(
            &bank,
            &Pubkey::default(),
            bank.get_slots_in_epoch(bank.epoch()) + 1,
        );
        // verify that there's inflation
        assert_ne!(bank1.capitalization(), bank.capitalization());

        // verify the inflation is represented in validator_points *
        let inflation = bank1.capitalization() - bank.capitalization();

        let rewards = bank1
            .get_account(&sysvar::rewards::id())
            .map(|account| Rewards::from_account(&account).unwrap())
            .unwrap();

        // verify the stake and vote accounts are the right size
        assert!(
            ((bank1.get_balance(&stake_id) - stake_account.lamports + bank1.get_balance(&vote_id)
                - vote_account.lamports) as f64
                - rewards.validator_point_value * validator_points as f64)
                .abs()
                < 1.0
        );

        // verify the rewards are the right size
        assert!(
            ((rewards.validator_point_value * validator_points as f64
                + rewards.storage_point_value * storage_points as f64)
                - inflation as f64)
                .abs()
                < 1.0 // rounding, truncating
        );

        // verify validator rewards show up in bank1.rewards vector
        // (currently storage rewards will not show up)
        assert_eq!(
            bank1.rewards,
            Some(vec![(
                stake_id,
                (rewards.validator_point_value * validator_points as f64) as i64
            )])
        );
    }
