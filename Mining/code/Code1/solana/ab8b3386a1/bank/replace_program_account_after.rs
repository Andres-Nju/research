    fn replace_program_account(
        &mut self,
        old_address: &Pubkey,
        new_address: &Pubkey,
        datapoint_name: &'static str,
    ) {
        if let Some(old_account) = self.get_account_with_fixed_root(old_address) {
            if let Some(new_account) = self.get_account_with_fixed_root(new_address) {
                datapoint_info!(datapoint_name, ("slot", self.slot, i64));

                // Burn lamports in the old account
                self.capitalization
                    .fetch_sub(old_account.lamports(), Relaxed);

                // Transfer new account to old account
                self.store_account(old_address, &new_account);

                // Clear new account
                self.store_account(new_address, &AccountSharedData::default());

                self.remove_executor(old_address);

                self.calculate_and_update_accounts_data_size_delta_off_chain(
                    old_account.data().len(),
                    new_account.data().len(),
                );
            }
        }
    }

    fn reconfigure_token2_native_mint(&mut self) {
        let reconfigure_token2_native_mint = match self.cluster_type() {
            ClusterType::Development => true,
            ClusterType::Devnet => true,
            ClusterType::Testnet => self.epoch() == 93,
            ClusterType::MainnetBeta => self.epoch() == 75,
        };

        if reconfigure_token2_native_mint {
            let mut native_mint_account = solana_sdk::account::AccountSharedData::from(Account {
                owner: inline_spl_token::id(),
                data: inline_spl_token::native_mint::ACCOUNT_DATA.to_vec(),
                lamports: sol_to_lamports(1.),
                executable: false,
                rent_epoch: self.epoch() + 1,
            });

            // As a workaround for
            // https://github.com/solana-labs/solana-program-library/issues/374, ensure that the
            // spl-token 2 native mint account is owned by the spl-token 2 program.
            let old_account_data_size;
            let store = if let Some(existing_native_mint_account) =
                self.get_account_with_fixed_root(&inline_spl_token::native_mint::id())
            {
                old_account_data_size = existing_native_mint_account.data().len();
                if existing_native_mint_account.owner() == &solana_sdk::system_program::id() {
                    native_mint_account.set_lamports(existing_native_mint_account.lamports());
                    true
                } else {
                    false
                }
            } else {
                old_account_data_size = 0;
                self.capitalization
                    .fetch_add(native_mint_account.lamports(), Relaxed);
                true
            };

            if store {
                self.store_account(&inline_spl_token::native_mint::id(), &native_mint_account);
                self.calculate_and_update_accounts_data_size_delta_off_chain(
                    old_account_data_size,
                    native_mint_account.data().len(),
                );
            }
        }
    }

    /// Get all the accounts for this bank and calculate stats
    pub fn get_total_accounts_stats(&self) -> ScanResult<TotalAccountsStats> {
        let accounts = self.get_all_accounts_with_modified_slots()?;
        Ok(self.calculate_total_accounts_stats(
            accounts
                .iter()
                .map(|(pubkey, account, _slot)| (pubkey, account)),
        ))
    }

    /// Given all the accounts for a bank, calculate stats
    pub fn calculate_total_accounts_stats<'a>(
        &self,
        accounts: impl Iterator<Item = (&'a Pubkey, &'a AccountSharedData)>,
    ) -> TotalAccountsStats {
        let rent_collector = self.rent_collector();
        let mut total_accounts_stats = TotalAccountsStats::default();
        accounts.for_each(|(pubkey, account)| {
            let data_len = account.data().len();
            total_accounts_stats.num_accounts += 1;
            total_accounts_stats.data_len += data_len;

            if account.executable() {
                total_accounts_stats.num_executable_accounts += 1;
                total_accounts_stats.executable_data_len += data_len;
            }

            if !rent_collector.should_collect_rent(pubkey, account)
                || rent_collector.get_rent_due(account).is_exempt()
            {
                total_accounts_stats.num_rent_exempt_accounts += 1;
            } else {
                total_accounts_stats.num_rent_paying_accounts += 1;
                total_accounts_stats.lamports_in_rent_paying_accounts += account.lamports();
                if data_len == 0 {
                    total_accounts_stats.num_rent_paying_accounts_without_data += 1;
                }
            }
        });

        total_accounts_stats
    }

    /// if we were to serialize THIS bank, what value should be saved for the prior accounts hash?
    /// This depends on the proximity to the time to take the snapshot and the time to use the snapshot.
    pub(crate) fn get_epoch_accounts_hash_to_serialize(&self) -> Option<Hash> {
        self.epoch_accounts_hash().map(|hash| *hash.as_ref())
    }

    /// Convenience fn to get the Epoch Accounts Hash
    fn epoch_accounts_hash(&self) -> Option<EpochAccountsHash> {
        *self
            .rc
            .accounts
            .accounts_db
            .epoch_accounts_hash
            .lock()
            .unwrap()
    }
}

/// Compute how much an account has changed size.  This function is useful when the data size delta
/// needs to be computed and passed to an `update_accounts_data_size_delta` function.
fn calculate_data_size_delta(old_data_size: usize, new_data_size: usize) -> i64 {
    assert!(old_data_size <= i64::MAX as usize);
    assert!(new_data_size <= i64::MAX as usize);
    let old_data_size = old_data_size as i64;
    let new_data_size = new_data_size as i64;

    new_data_size.saturating_sub(old_data_size)
}

/// Since `apply_feature_activations()` has different behavior depending on its caller, enumerate
/// those callers explicitly.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum ApplyFeatureActivationsCaller {
    FinishInit,
    NewFromParent,
    WarpFromParent,
}

/// Return the computed values from `collect_rent_from_accounts()`
///
/// Since `collect_rent_from_accounts()` is running in parallel, instead of updating the
/// atomics/shared data inside this function, return those values in this struct for the caller to
/// process later.
#[derive(Debug, Default)]
struct CollectRentFromAccountsInfo {
    rent_collected_info: CollectedInfo,
    rent_rewards: Vec<(Pubkey, RewardInfo)>,
    rewrites_skipped: Vec<(Pubkey, Hash)>,
    time_collecting_rent_us: u64,
    time_hashing_skipped_rewrites_us: u64,
    time_storing_accounts_us: u64,
    num_accounts: usize,
}

/// Return the computed values—of each iteration in the parallel loop inside
/// `collect_rent_in_partition()`—and then perform a reduce on all of them.
#[derive(Debug, Default)]
struct CollectRentInPartitionInfo {
    rent_collected: u64,
    accounts_data_size_reclaimed: u64,
    rent_rewards: Vec<(Pubkey, RewardInfo)>,
    rewrites_skipped: Vec<(Pubkey, Hash)>,
    time_loading_accounts_us: u64,
    time_collecting_rent_us: u64,
    time_hashing_skipped_rewrites_us: u64,
    time_storing_accounts_us: u64,
    num_accounts: usize,
}

impl CollectRentInPartitionInfo {
    /// Create a new `CollectRentInPartitionInfo` from the results of loading accounts and
    /// collecting rent on them.
    #[must_use]
    fn new(info: CollectRentFromAccountsInfo, time_loading_accounts: Duration) -> Self {
        Self {
            rent_collected: info.rent_collected_info.rent_amount,
            accounts_data_size_reclaimed: info.rent_collected_info.account_data_len_reclaimed,
            rent_rewards: info.rent_rewards,
            rewrites_skipped: info.rewrites_skipped,
            time_loading_accounts_us: time_loading_accounts.as_micros() as u64,
            time_collecting_rent_us: info.time_collecting_rent_us,
            time_hashing_skipped_rewrites_us: info.time_hashing_skipped_rewrites_us,
            time_storing_accounts_us: info.time_storing_accounts_us,
            num_accounts: info.num_accounts,
        }
    }

    /// Reduce (i.e. 'combine') two `CollectRentInPartitionInfo`s into one.
    ///
    /// This fn is used by `collect_rent_in_partition()` as the reduce step (of map-reduce) in its
    /// parallel loop of rent collection.
    #[must_use]
    fn reduce(lhs: Self, rhs: Self) -> Self {
        Self {
            rent_collected: lhs.rent_collected.saturating_add(rhs.rent_collected),
            accounts_data_size_reclaimed: lhs
                .accounts_data_size_reclaimed
                .saturating_add(rhs.accounts_data_size_reclaimed),
            rent_rewards: [lhs.rent_rewards, rhs.rent_rewards].concat(),
            rewrites_skipped: [lhs.rewrites_skipped, rhs.rewrites_skipped].concat(),
            time_loading_accounts_us: lhs
                .time_loading_accounts_us
                .saturating_add(rhs.time_loading_accounts_us),
            time_collecting_rent_us: lhs
                .time_collecting_rent_us
                .saturating_add(rhs.time_collecting_rent_us),
            time_hashing_skipped_rewrites_us: lhs
                .time_hashing_skipped_rewrites_us
                .saturating_add(rhs.time_hashing_skipped_rewrites_us),
            time_storing_accounts_us: lhs
                .time_storing_accounts_us
                .saturating_add(rhs.time_storing_accounts_us),
            num_accounts: lhs.num_accounts.saturating_add(rhs.num_accounts),
        }
    }
}

/// Struct to collect stats when scanning all accounts in `get_total_accounts_stats()`
#[derive(Debug, Default, Copy, Clone)]
pub struct TotalAccountsStats {
    /// Total number of accounts
    pub num_accounts: usize,
    /// Total data size of all accounts
    pub data_len: usize,

    /// Total number of executable accounts
    pub num_executable_accounts: usize,
    /// Total data size of executable accounts
    pub executable_data_len: usize,

    /// Total number of rent exempt accounts
    pub num_rent_exempt_accounts: usize,
    /// Total number of rent paying accounts
    pub num_rent_paying_accounts: usize,
    /// Total number of rent paying accounts without data
    pub num_rent_paying_accounts_without_data: usize,
    /// Total amount of lamports in rent paying accounts
    pub lamports_in_rent_paying_accounts: u64,
}

impl Drop for Bank {
    fn drop(&mut self) {
        if let Some(drop_callback) = self.drop_callback.read().unwrap().0.as_ref() {
            drop_callback.callback(self);
        } else {
            // Default case for tests
            self.rc
                .accounts
                .accounts_db
                .purge_slot(self.slot(), self.bank_id(), false);
        }
    }
}

/// utility function used for testing and benchmarking.
pub mod test_utils {
    use {super::Bank, solana_sdk::hash::hashv};
    pub fn goto_end_of_slot(bank: &mut Bank) {
        let mut tick_hash = bank.last_blockhash();
        loop {
            tick_hash = hashv(&[tick_hash.as_ref(), &[42]]);
            bank.register_tick(&tick_hash);
            if tick_hash == bank.last_blockhash() {
                bank.freeze();
                return;
            }
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    #[allow(deprecated)]
    use solana_sdk::sysvar::fees::Fees;
    use {
        super::*,
        crate::{
            accounts_background_service::{PrunedBanksRequestHandler, SendDroppedBankCallback},
            accounts_db::DEFAULT_ACCOUNTS_SHRINK_RATIO,
            accounts_index::{AccountIndex, AccountSecondaryIndexes, ScanError, ITER_BATCH_SIZE},
            ancestors::Ancestors,
            genesis_utils::{
                self, activate_all_features, bootstrap_validator_stake_lamports,
                create_genesis_config_with_leader, create_genesis_config_with_vote_accounts,
                genesis_sysvar_and_builtin_program_lamports, GenesisConfigInfo,
                ValidatorVoteKeypairs,
            },
            rent_paying_accounts_by_partition::RentPayingAccountsByPartition,
            status_cache::MAX_CACHE_ENTRIES,
        },
        crossbeam_channel::{bounded, unbounded},
        solana_program_runtime::{
            compute_budget::MAX_COMPUTE_UNIT_LIMIT,
            invoke_context::InvokeContext,
            prioritization_fee::{PrioritizationFeeDetails, PrioritizationFeeType},
        },
        solana_sdk::{
            account::Account,
            bpf_loader, bpf_loader_deprecated, bpf_loader_upgradeable,
            clock::{DEFAULT_SLOTS_PER_EPOCH, DEFAULT_TICKS_PER_SLOT, MAX_RECENT_BLOCKHASHES},
            compute_budget::ComputeBudgetInstruction,
            entrypoint::MAX_PERMITTED_DATA_INCREASE,
            epoch_schedule::MINIMUM_SLOTS_PER_EPOCH,
            feature::Feature,
            genesis_config::create_genesis_config,
            hash,
            instruction::{AccountMeta, CompiledInstruction, Instruction, InstructionError},
            message::{Message, MessageHeader},
            native_token::LAMPORTS_PER_SOL,
            nonce,
            poh_config::PohConfig,
            program::MAX_RETURN_DATA,
            rent::Rent,
            signature::{keypair_from_seed, Keypair, Signer},
            stake::{
                instruction as stake_instruction,
                state::{Authorized, Delegation, Lockup, Stake},
            },
            system_instruction::{
                self, SystemError, MAX_PERMITTED_ACCOUNTS_DATA_ALLOCATIONS_PER_TRANSACTION,
                MAX_PERMITTED_DATA_LENGTH,
            },
            system_program,
            timing::duration_as_s,
            transaction_context::IndexOfAccount,
        },
        solana_vote_program::{
            vote_instruction,
            vote_state::{
                self, BlockTimestamp, Vote, VoteInit, VoteState, VoteStateVersions,
                MAX_LOCKOUT_HISTORY,
            },
        },
        std::{result, str::FromStr, thread::Builder, time::Duration},
        test_utils::goto_end_of_slot,
    };

    fn new_sanitized_message(
        instructions: &[Instruction],
        payer: Option<&Pubkey>,
    ) -> SanitizedMessage {
        Message::new(instructions, payer).try_into().unwrap()
    }

    fn new_execution_result(
        status: Result<()>,
        nonce: Option<&NonceFull>,
    ) -> TransactionExecutionResult {
        TransactionExecutionResult::Executed {
            details: TransactionExecutionDetails {
                status,
                log_messages: None,
                inner_instructions: None,
                durable_nonce_fee: nonce.map(DurableNonceFee::from),
                return_data: None,
                executed_units: 0,
                accounts_data_len_delta: 0,
            },
            executors: Rc::new(RefCell::new(Executors::default())),
        }
    }

    impl Bank {
        fn clean_accounts_for_tests(&self) {
            self.rc.accounts.accounts_db.clean_accounts_for_tests()
        }
    }

    #[test]
    fn test_nonce_info() {
        let lamports_per_signature = 42;

        let nonce_authority = keypair_from_seed(&[0; 32]).unwrap();
        let nonce_address = nonce_authority.pubkey();
        let from = keypair_from_seed(&[1; 32]).unwrap();
        let from_address = from.pubkey();
        let to_address = Pubkey::new_unique();

        let durable_nonce = DurableNonce::from_blockhash(&Hash::new_unique());
        let nonce_account = AccountSharedData::new_data(
            43,
            &nonce::state::Versions::new(nonce::State::Initialized(nonce::state::Data::new(
                Pubkey::default(),
                durable_nonce,
                lamports_per_signature,
            ))),
            &system_program::id(),
        )
        .unwrap();
        let from_account = AccountSharedData::new(44, 0, &Pubkey::default());
        let to_account = AccountSharedData::new(45, 0, &Pubkey::default());
        let recent_blockhashes_sysvar_account = AccountSharedData::new(4, 0, &Pubkey::default());

        const TEST_RENT_DEBIT: u64 = 1;
        let rent_collected_nonce_account = {
            let mut account = nonce_account.clone();
            account.set_lamports(nonce_account.lamports() - TEST_RENT_DEBIT);
            account
        };
        let rent_collected_from_account = {
            let mut account = from_account.clone();
            account.set_lamports(from_account.lamports() - TEST_RENT_DEBIT);
            account
        };

        let instructions = vec![
            system_instruction::advance_nonce_account(&nonce_address, &nonce_authority.pubkey()),
            system_instruction::transfer(&from_address, &to_address, 42),
        ];

        // NoncePartial create + NonceInfo impl
        let partial = NoncePartial::new(nonce_address, rent_collected_nonce_account.clone());
        assert_eq!(*partial.address(), nonce_address);
        assert_eq!(*partial.account(), rent_collected_nonce_account);
        assert_eq!(
            partial.lamports_per_signature(),
            Some(lamports_per_signature)
        );
        assert_eq!(partial.fee_payer_account(), None);

        // Add rent debits to ensure the rollback captures accounts without rent fees
        let mut rent_debits = RentDebits::default();
        rent_debits.insert(
            &from_address,
            TEST_RENT_DEBIT,
            rent_collected_from_account.lamports(),
        );
        rent_debits.insert(
            &nonce_address,
            TEST_RENT_DEBIT,
            rent_collected_nonce_account.lamports(),
        );

        // NonceFull create + NonceInfo impl
        {
            let message = new_sanitized_message(&instructions, Some(&from_address));
            let accounts = [
                (
                    *message.account_keys().get(0).unwrap(),
                    rent_collected_from_account.clone(),
                ),
                (
                    *message.account_keys().get(1).unwrap(),
                    rent_collected_nonce_account.clone(),
                ),
                (*message.account_keys().get(2).unwrap(), to_account.clone()),
                (
                    *message.account_keys().get(3).unwrap(),
                    recent_blockhashes_sysvar_account.clone(),
                ),
            ];

            let full = NonceFull::from_partial(partial.clone(), &message, &accounts, &rent_debits)
                .unwrap();
            assert_eq!(*full.address(), nonce_address);
            assert_eq!(*full.account(), rent_collected_nonce_account);
            assert_eq!(full.lamports_per_signature(), Some(lamports_per_signature));
            assert_eq!(
                full.fee_payer_account(),
                Some(&from_account),
                "rent debit should be refunded in captured fee account"
            );
        }

        // Nonce account is fee-payer
        {
            let message = new_sanitized_message(&instructions, Some(&nonce_address));
            let accounts = [
                (
                    *message.account_keys().get(0).unwrap(),
                    rent_collected_nonce_account,
                ),
                (
                    *message.account_keys().get(1).unwrap(),
                    rent_collected_from_account,
                ),
                (*message.account_keys().get(2).unwrap(), to_account),
                (
                    *message.account_keys().get(3).unwrap(),
                    recent_blockhashes_sysvar_account,
                ),
            ];

            let full = NonceFull::from_partial(partial.clone(), &message, &accounts, &rent_debits)
                .unwrap();
            assert_eq!(*full.address(), nonce_address);
            assert_eq!(*full.account(), nonce_account);
            assert_eq!(full.lamports_per_signature(), Some(lamports_per_signature));
            assert_eq!(full.fee_payer_account(), None);
        }

        // NonceFull create, fee-payer not in account_keys fails
        {
            let message = new_sanitized_message(&instructions, Some(&nonce_address));
            assert_eq!(
                NonceFull::from_partial(partial, &message, &[], &RentDebits::default())
                    .unwrap_err(),
                TransactionError::AccountNotFound,
            );
        }
    }

    #[test]
    fn test_bank_unix_timestamp_from_genesis() {
        let (genesis_config, _mint_keypair) = create_genesis_config(1);
        let mut bank = Arc::new(Bank::new_for_tests(&genesis_config));

        assert_eq!(
            genesis_config.creation_time,
            bank.unix_timestamp_from_genesis()
        );
        let slots_per_sec = 1.0
            / (duration_as_s(&genesis_config.poh_config.target_tick_duration)
                * genesis_config.ticks_per_slot as f32);

        for _i in 0..slots_per_sec as usize + 1 {
            bank = Arc::new(new_from_parent(&bank));
        }

        assert!(bank.unix_timestamp_from_genesis() - genesis_config.creation_time >= 1);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn test_bank_new() {
        let dummy_leader_pubkey = solana_sdk::pubkey::new_rand();
        let dummy_leader_stake_lamports = bootstrap_validator_stake_lamports();
        let mint_lamports = 10_000;
        let GenesisConfigInfo {
            mut genesis_config,
            mint_keypair,
            voting_keypair,
            ..
        } = create_genesis_config_with_leader(
            mint_lamports,
            &dummy_leader_pubkey,
            dummy_leader_stake_lamports,
        );

        genesis_config.rent = Rent {
            lamports_per_byte_year: 5,
            exemption_threshold: 1.2,
            burn_percent: 5,
        };

        let bank = Bank::new_for_tests(&genesis_config);
        assert_eq!(bank.get_balance(&mint_keypair.pubkey()), mint_lamports);
        assert_eq!(
            bank.get_balance(&voting_keypair.pubkey()),
            dummy_leader_stake_lamports /* 1 token goes to the vote account associated with dummy_leader_lamports */
        );

        let rent_account = bank.get_account(&sysvar::rent::id()).unwrap();
        let rent = from_account::<sysvar::rent::Rent, _>(&rent_account).unwrap();

        assert_eq!(rent.burn_percent, 5);
        assert_eq!(rent.exemption_threshold, 1.2);
        assert_eq!(rent.lamports_per_byte_year, 5);
    }

    fn create_simple_test_bank(lamports: u64) -> Bank {
        let (genesis_config, _mint_keypair) = create_genesis_config(lamports);
        Bank::new_for_tests(&genesis_config)
    }

    fn create_simple_test_arc_bank(lamports: u64) -> Arc<Bank> {
        Arc::new(create_simple_test_bank(lamports))
    }

    #[test]
    fn test_bank_block_height() {
        let bank0 = create_simple_test_arc_bank(1);
        assert_eq!(bank0.block_height(), 0);
        let bank1 = Arc::new(new_from_parent(&bank0));
        assert_eq!(bank1.block_height(), 1);
    }

    #[test]
    fn test_bank_update_epoch_stakes() {
        impl Bank {
            fn epoch_stake_keys(&self) -> Vec<Epoch> {
                let mut keys: Vec<Epoch> = self.epoch_stakes.keys().copied().collect();
                keys.sort_unstable();
                keys
            }

            fn epoch_stake_key_info(&self) -> (Epoch, Epoch, usize) {
                let mut keys: Vec<Epoch> = self.epoch_stakes.keys().copied().collect();
                keys.sort_unstable();
                (*keys.first().unwrap(), *keys.last().unwrap(), keys.len())
            }
        }

        let mut bank = create_simple_test_bank(100_000);

        let initial_epochs = bank.epoch_stake_keys();
        assert_eq!(initial_epochs, vec![0, 1]);

        for existing_epoch in &initial_epochs {
            bank.update_epoch_stakes(*existing_epoch);
            assert_eq!(bank.epoch_stake_keys(), initial_epochs);
        }

        for epoch in (initial_epochs.len() as Epoch)..MAX_LEADER_SCHEDULE_STAKES {
            bank.update_epoch_stakes(epoch);
            assert_eq!(bank.epoch_stakes.len() as Epoch, epoch + 1);
        }

        assert_eq!(
            bank.epoch_stake_key_info(),
            (
                0,
                MAX_LEADER_SCHEDULE_STAKES - 1,
                MAX_LEADER_SCHEDULE_STAKES as usize
            )
        );

        bank.update_epoch_stakes(MAX_LEADER_SCHEDULE_STAKES);
        assert_eq!(
            bank.epoch_stake_key_info(),
            (
                0,
                MAX_LEADER_SCHEDULE_STAKES,
                MAX_LEADER_SCHEDULE_STAKES as usize + 1
            )
        );

        bank.update_epoch_stakes(MAX_LEADER_SCHEDULE_STAKES + 1);
        assert_eq!(
            bank.epoch_stake_key_info(),
            (
                1,
                MAX_LEADER_SCHEDULE_STAKES + 1,
                MAX_LEADER_SCHEDULE_STAKES as usize + 1
            )
        );
    }

    fn bank0_sysvar_delta() -> u64 {
        const SLOT_HISTORY_SYSVAR_MIN_BALANCE: u64 = 913_326_000;
        SLOT_HISTORY_SYSVAR_MIN_BALANCE
    }

    fn bank1_sysvar_delta() -> u64 {
        const SLOT_HASHES_SYSVAR_MIN_BALANCE: u64 = 143_487_360;
        SLOT_HASHES_SYSVAR_MIN_BALANCE
    }

    #[test]
    fn test_bank_capitalization() {
        let bank0 = Arc::new(Bank::new_for_tests(&GenesisConfig {
            accounts: (0..42)
                .map(|_| {
                    (
                        solana_sdk::pubkey::new_rand(),
                        Account::new(42, 0, &Pubkey::default()),
                    )
                })
                .collect(),
            cluster_type: ClusterType::MainnetBeta,
            ..GenesisConfig::default()
        }));

        assert_eq!(
            bank0.capitalization(),
            42 * 42 + genesis_sysvar_and_builtin_program_lamports(),
        );

        bank0.freeze();

        assert_eq!(
            bank0.capitalization(),
            42 * 42 + genesis_sysvar_and_builtin_program_lamports() + bank0_sysvar_delta(),
        );

        let bank1 = Bank::new_from_parent(&bank0, &Pubkey::default(), 1);
        assert_eq!(
            bank1.capitalization(),
            42 * 42
                + genesis_sysvar_and_builtin_program_lamports()
                + bank0_sysvar_delta()
                + bank1_sysvar_delta(),
        );
    }

    fn rent_with_exemption_threshold(exemption_threshold: f64) -> Rent {
        Rent {
            lamports_per_byte_year: 1,
            exemption_threshold,
            burn_percent: 10,
        }
    }

    #[test]
    /// one thing being tested here is that a failed tx (due to rent collection using up all lamports) followed by rent collection
    /// results in the same state as if just rent collection ran (and emptied the accounts that have too few lamports)
    fn test_credit_debit_rent_no_side_effect_on_hash() {
        solana_logger::setup();

        let (mut genesis_config, _mint_keypair) = create_genesis_config(10);

        genesis_config.rent = rent_with_exemption_threshold(21.0);

        let slot = years_as_slots(
            2.0,
            &genesis_config.poh_config.target_tick_duration,
            genesis_config.ticks_per_slot,
        ) as u64;
        let root_bank = Arc::new(Bank::new_for_tests(&genesis_config));
        let bank = Bank::new_from_parent(&root_bank, &Pubkey::default(), slot);

        let root_bank_2 = Arc::new(Bank::new_for_tests(&genesis_config));
        let bank_with_success_txs = Bank::new_from_parent(&root_bank_2, &Pubkey::default(), slot);

        assert_eq!(bank.last_blockhash(), genesis_config.hash());

        let plenty_of_lamports = 264;
        let too_few_lamports = 10;
        // Initialize credit-debit and credit only accounts
        let accounts = [
            AccountSharedData::new(plenty_of_lamports, 0, &Pubkey::default()),
            AccountSharedData::new(plenty_of_lamports, 1, &Pubkey::default()),
            AccountSharedData::new(plenty_of_lamports, 0, &Pubkey::default()),
            AccountSharedData::new(plenty_of_lamports, 1, &Pubkey::default()),
            // Transaction between these two accounts will fail
            AccountSharedData::new(too_few_lamports, 0, &Pubkey::default()),
            AccountSharedData::new(too_few_lamports, 1, &Pubkey::default()),
        ];

        let keypairs = accounts.iter().map(|_| Keypair::new()).collect::<Vec<_>>();
        {
            // make sure rent and epoch change are such that we collect all lamports in accounts 4 & 5
            let mut account_copy = accounts[4].clone();
            let expected_rent = bank.rent_collector().collect_from_existing_account(
                &keypairs[4].pubkey(),
                &mut account_copy,
                None,
                true, // preserve_rent_epoch_for_rent_exempt_accounts
            );
            assert_eq!(expected_rent.rent_amount, too_few_lamports);
            assert_eq!(account_copy.lamports(), 0);
        }

        for i in 0..accounts.len() {
            let account = &accounts[i];
            bank.store_account(&keypairs[i].pubkey(), account);
            bank_with_success_txs.store_account(&keypairs[i].pubkey(), account);
        }

        // Make builtin instruction loader rent exempt
        let system_program_id = system_program::id();
        let mut system_program_account = bank.get_account(&system_program_id).unwrap();
        system_program_account.set_lamports(
            bank.get_minimum_balance_for_rent_exemption(system_program_account.data().len()),
        );
        bank.store_account(&system_program_id, &system_program_account);
        bank_with_success_txs.store_account(&system_program_id, &system_program_account);

        let t1 = system_transaction::transfer(
            &keypairs[0],
            &keypairs[1].pubkey(),
            1,
            genesis_config.hash(),
        );
        let t2 = system_transaction::transfer(
            &keypairs[2],
            &keypairs[3].pubkey(),
            1,
            genesis_config.hash(),
        );
        // the idea is this transaction will result in both accounts being drained of all lamports due to rent collection
        let t3 = system_transaction::transfer(
            &keypairs[4],
            &keypairs[5].pubkey(),
            1,
            genesis_config.hash(),
        );

        let txs = vec![t1.clone(), t2.clone(), t3];
        let res = bank.process_transactions(txs.iter());

        assert_eq!(res.len(), 3);
        assert_eq!(res[0], Ok(()));
        assert_eq!(res[1], Ok(()));
        assert_eq!(res[2], Err(TransactionError::AccountNotFound));

        bank.freeze();

        let rwlockguard_bank_hash = bank.hash.read().unwrap();
        let bank_hash = rwlockguard_bank_hash.as_ref();

        let txs = vec![t2, t1];
        let res = bank_with_success_txs.process_transactions(txs.iter());

        assert_eq!(res.len(), 2);
        assert_eq!(res[0], Ok(()));
        assert_eq!(res[1], Ok(()));

        bank_with_success_txs.freeze();

        let rwlockguard_bank_with_success_txs_hash = bank_with_success_txs.hash.read().unwrap();
        let bank_with_success_txs_hash = rwlockguard_bank_with_success_txs_hash.as_ref();

        assert_eq!(bank_with_success_txs_hash, bank_hash);
    }

    fn store_accounts_for_rent_test(
        bank: &Bank,
        keypairs: &mut [Keypair],
        mock_program_id: Pubkey,
        generic_rent_due_for_system_account: u64,
    ) {
        let mut account_pairs: Vec<TransactionAccount> = Vec::with_capacity(keypairs.len() - 1);
        account_pairs.push((
            keypairs[0].pubkey(),
            AccountSharedData::new(
                generic_rent_due_for_system_account + 2,
                0,
                &Pubkey::default(),
            ),
        ));
        account_pairs.push((
            keypairs[1].pubkey(),
            AccountSharedData::new(
                generic_rent_due_for_system_account + 2,
                0,
                &Pubkey::default(),
            ),
        ));
        account_pairs.push((
            keypairs[2].pubkey(),
            AccountSharedData::new(
                generic_rent_due_for_system_account + 2,
                0,
                &Pubkey::default(),
            ),
        ));
        account_pairs.push((
            keypairs[3].pubkey(),
            AccountSharedData::new(
                generic_rent_due_for_system_account + 2,
                0,
                &Pubkey::default(),
            ),
        ));
        account_pairs.push((
            keypairs[4].pubkey(),
            AccountSharedData::new(10, 0, &Pubkey::default()),
        ));
        account_pairs.push((
            keypairs[5].pubkey(),
            AccountSharedData::new(10, 0, &Pubkey::default()),
        ));
        account_pairs.push((
            keypairs[6].pubkey(),
            AccountSharedData::new(
                (2 * generic_rent_due_for_system_account) + 24,
                0,
                &Pubkey::default(),
            ),
        ));

        account_pairs.push((
            keypairs[8].pubkey(),
            AccountSharedData::new(
                generic_rent_due_for_system_account + 2 + 929,
                0,
                &Pubkey::default(),
            ),
        ));
        account_pairs.push((
            keypairs[9].pubkey(),
            AccountSharedData::new(10, 0, &Pubkey::default()),
        ));

        // Feeding to MockProgram to test read only rent behaviour
        account_pairs.push((
            keypairs[10].pubkey(),
            AccountSharedData::new(
                generic_rent_due_for_system_account + 3,
                0,
                &Pubkey::default(),
            ),
        ));
        account_pairs.push((
            keypairs[11].pubkey(),
            AccountSharedData::new(generic_rent_due_for_system_account + 3, 0, &mock_program_id),
        ));
        account_pairs.push((
            keypairs[12].pubkey(),
            AccountSharedData::new(generic_rent_due_for_system_account + 3, 0, &mock_program_id),
        ));
        account_pairs.push((
            keypairs[13].pubkey(),
            AccountSharedData::new(14, 22, &mock_program_id),
        ));

        for account_pair in account_pairs.iter() {
            bank.store_account(&account_pair.0, &account_pair.1);
        }
    }

    fn create_child_bank_for_rent_test(
        root_bank: &Arc<Bank>,
        genesis_config: &GenesisConfig,
    ) -> Bank {
        let mut bank = Bank::new_from_parent(
            root_bank,
            &Pubkey::default(),
            years_as_slots(
                2.0,
                &genesis_config.poh_config.target_tick_duration,
                genesis_config.ticks_per_slot,
            ) as u64,
        );
        bank.rent_collector.slots_per_year = 421_812.0;
        bank
    }

    fn assert_capitalization_diff(bank: &Bank, updater: impl Fn(), asserter: impl Fn(u64, u64)) {
        let old = bank.capitalization();
        updater();
        let new = bank.capitalization();
        asserter(old, new);
        assert_eq!(bank.capitalization(), bank.calculate_capitalization(true));
    }

    #[test]
    fn test_store_account_and_update_capitalization_missing() {
        let bank = create_simple_test_bank(0);
        let pubkey = solana_sdk::pubkey::new_rand();

        let some_lamports = 400;
        let account = AccountSharedData::new(some_lamports, 0, &system_program::id());

        assert_capitalization_diff(
            &bank,
            || bank.store_account_and_update_capitalization(&pubkey, &account),
            |old, new| assert_eq!(old + some_lamports, new),
        );
        assert_eq!(account, bank.get_account(&pubkey).unwrap());
    }

    #[test]
    fn test_store_account_and_update_capitalization_increased() {
        let old_lamports = 400;
        let (genesis_config, mint_keypair) = create_genesis_config(old_lamports);
        let bank = Bank::new_for_tests(&genesis_config);
        let pubkey = mint_keypair.pubkey();

        let new_lamports = 500;
        let account = AccountSharedData::new(new_lamports, 0, &system_program::id());

        assert_capitalization_diff(
            &bank,
            || bank.store_account_and_update_capitalization(&pubkey, &account),
            |old, new| assert_eq!(old + 100, new),
        );
        assert_eq!(account, bank.get_account(&pubkey).unwrap());
    }

    #[test]
    fn test_store_account_and_update_capitalization_decreased() {
        let old_lamports = 400;
        let (genesis_config, mint_keypair) = create_genesis_config(old_lamports);
        let bank = Bank::new_for_tests(&genesis_config);
        let pubkey = mint_keypair.pubkey();

        let new_lamports = 100;
        let account = AccountSharedData::new(new_lamports, 0, &system_program::id());

        assert_capitalization_diff(
            &bank,
            || bank.store_account_and_update_capitalization(&pubkey, &account),
            |old, new| assert_eq!(old - 300, new),
        );
        assert_eq!(account, bank.get_account(&pubkey).unwrap());
    }

    #[test]
    fn test_store_account_and_update_capitalization_unchanged() {
        let lamports = 400;
        let (genesis_config, mint_keypair) = create_genesis_config(lamports);
        let bank = Bank::new_for_tests(&genesis_config);
        let pubkey = mint_keypair.pubkey();

        let account = AccountSharedData::new(lamports, 1, &system_program::id());

        assert_capitalization_diff(
            &bank,
            || bank.store_account_and_update_capitalization(&pubkey, &account),
            |old, new| assert_eq!(old, new),
        );
        assert_eq!(account, bank.get_account(&pubkey).unwrap());
    }

    #[test]
    #[ignore]
    fn test_rent_distribution() {
        solana_logger::setup();

        let bootstrap_validator_pubkey = solana_sdk::pubkey::new_rand();
        let bootstrap_validator_stake_lamports = 30;
        let mut genesis_config = create_genesis_config_with_leader(
            10,
            &bootstrap_validator_pubkey,
            bootstrap_validator_stake_lamports,
        )
        .genesis_config;
        // While we are preventing new accounts left in a rent-paying state, not quite ready to rip
        // out all the rent assessment tests. Just deactivate the feature for now.
        genesis_config
            .accounts
            .remove(&feature_set::require_rent_exempt_accounts::id())
            .unwrap();

        genesis_config.epoch_schedule = EpochSchedule::custom(
            MINIMUM_SLOTS_PER_EPOCH,
            genesis_config.epoch_schedule.leader_schedule_slot_offset,
            false,
        );

        genesis_config.rent = rent_with_exemption_threshold(2.0);

        let rent = Rent::free();

        let validator_1_pubkey = solana_sdk::pubkey::new_rand();
        let validator_1_stake_lamports = 20;
        let validator_1_staking_keypair = Keypair::new();
        let validator_1_voting_keypair = Keypair::new();

        let validator_1_vote_account = vote_state::create_account(
            &validator_1_voting_keypair.pubkey(),
            &validator_1_pubkey,
            0,
            validator_1_stake_lamports,
        );

        let validator_1_stake_account = stake_state::create_account(
            &validator_1_staking_keypair.pubkey(),
            &validator_1_voting_keypair.pubkey(),
            &validator_1_vote_account,
            &rent,
            validator_1_stake_lamports,
        );

        genesis_config.accounts.insert(
            validator_1_pubkey,
            Account::new(42, 0, &system_program::id()),
        );
        genesis_config.accounts.insert(
            validator_1_staking_keypair.pubkey(),
            Account::from(validator_1_stake_account),
        );
        genesis_config.accounts.insert(
            validator_1_voting_keypair.pubkey(),
            Account::from(validator_1_vote_account),
        );

        let validator_2_pubkey = solana_sdk::pubkey::new_rand();
        let validator_2_stake_lamports = 20;
        let validator_2_staking_keypair = Keypair::new();
        let validator_2_voting_keypair = Keypair::new();

        let validator_2_vote_account = vote_state::create_account(
            &validator_2_voting_keypair.pubkey(),
            &validator_2_pubkey,
            0,
            validator_2_stake_lamports,
        );

        let validator_2_stake_account = stake_state::create_account(
            &validator_2_staking_keypair.pubkey(),
            &validator_2_voting_keypair.pubkey(),
            &validator_2_vote_account,
            &rent,
            validator_2_stake_lamports,
        );

        genesis_config.accounts.insert(
            validator_2_pubkey,
            Account::new(42, 0, &system_program::id()),
        );
        genesis_config.accounts.insert(
            validator_2_staking_keypair.pubkey(),
            Account::from(validator_2_stake_account),
        );
        genesis_config.accounts.insert(
            validator_2_voting_keypair.pubkey(),
            Account::from(validator_2_vote_account),
        );

        let validator_3_pubkey = solana_sdk::pubkey::new_rand();
        let validator_3_stake_lamports = 30;
        let validator_3_staking_keypair = Keypair::new();
        let validator_3_voting_keypair = Keypair::new();

        let validator_3_vote_account = vote_state::create_account(
            &validator_3_voting_keypair.pubkey(),
            &validator_3_pubkey,
            0,
            validator_3_stake_lamports,
        );

        let validator_3_stake_account = stake_state::create_account(
            &validator_3_staking_keypair.pubkey(),
            &validator_3_voting_keypair.pubkey(),
            &validator_3_vote_account,
            &rent,
            validator_3_stake_lamports,
        );

        genesis_config.accounts.insert(
            validator_3_pubkey,
            Account::new(42, 0, &system_program::id()),
        );
        genesis_config.accounts.insert(
            validator_3_staking_keypair.pubkey(),
            Account::from(validator_3_stake_account),
        );
        genesis_config.accounts.insert(
            validator_3_voting_keypair.pubkey(),
            Account::from(validator_3_vote_account),
        );

        genesis_config.rent = rent_with_exemption_threshold(10.0);

        let mut bank = Bank::new_for_tests(&genesis_config);
        // Enable rent collection
        bank.rent_collector.epoch = 5;
        bank.rent_collector.slots_per_year = 192.0;

        let payer = Keypair::new();
        let payer_account = AccountSharedData::new(400, 0, &system_program::id());
        bank.store_account_and_update_capitalization(&payer.pubkey(), &payer_account);

        let payee = Keypair::new();
        let payee_account = AccountSharedData::new(70, 1, &system_program::id());
        bank.store_account_and_update_capitalization(&payee.pubkey(), &payee_account);

        let bootstrap_validator_initial_balance = bank.get_balance(&bootstrap_validator_pubkey);

        let tx = system_transaction::transfer(&payer, &payee.pubkey(), 180, genesis_config.hash());

        let result = bank.process_transaction(&tx);
        assert_eq!(result, Ok(()));

        let mut total_rent_deducted = 0;

        // 400 - 128(Rent) - 180(Transfer)
        assert_eq!(bank.get_balance(&payer.pubkey()), 92);
        total_rent_deducted += 128;

        // 70 - 70(Rent) + 180(Transfer) - 21(Rent)
        assert_eq!(bank.get_balance(&payee.pubkey()), 159);
        total_rent_deducted += 70 + 21;

        let previous_capitalization = bank.capitalization.load(Relaxed);

        bank.freeze();

        assert_eq!(bank.collected_rent.load(Relaxed), total_rent_deducted);

        let burned_portion =
            total_rent_deducted * u64::from(bank.rent_collector.rent.burn_percent) / 100;
        let rent_to_be_distributed = total_rent_deducted - burned_portion;

        let bootstrap_validator_portion =
            ((bootstrap_validator_stake_lamports * rent_to_be_distributed) as f64 / 100.0) as u64
                + 1; // Leftover lamport
        assert_eq!(
            bank.get_balance(&bootstrap_validator_pubkey),
            bootstrap_validator_portion + bootstrap_validator_initial_balance
        );

        // Since, validator 1 and validator 2 has equal smallest stake, it comes down to comparison
        // between their pubkey.
        let tweak_1 = if validator_1_pubkey > validator_2_pubkey {
            1
        } else {
            0
        };
        let validator_1_portion =
            ((validator_1_stake_lamports * rent_to_be_distributed) as f64 / 100.0) as u64 + tweak_1;
        assert_eq!(
            bank.get_balance(&validator_1_pubkey),
            validator_1_portion + 42 - tweak_1,
        );

        // Since, validator 1 and validator 2 has equal smallest stake, it comes down to comparison
        // between their pubkey.
        let tweak_2 = if validator_2_pubkey > validator_1_pubkey {
            1
        } else {
            0
        };
        let validator_2_portion =
            ((validator_2_stake_lamports * rent_to_be_distributed) as f64 / 100.0) as u64 + tweak_2;
        assert_eq!(
            bank.get_balance(&validator_2_pubkey),
            validator_2_portion + 42 - tweak_2,
        );

        let validator_3_portion =
            ((validator_3_stake_lamports * rent_to_be_distributed) as f64 / 100.0) as u64 + 1;
        assert_eq!(
            bank.get_balance(&validator_3_pubkey),
            validator_3_portion + 42
        );

        let current_capitalization = bank.capitalization.load(Relaxed);

        // only slot history is newly created
        let sysvar_and_builtin_program_delta =
            min_rent_exempt_balance_for_sysvars(&bank, &[sysvar::slot_history::id()]);
        assert_eq!(
            previous_capitalization - (current_capitalization - sysvar_and_builtin_program_delta),
            burned_portion
        );

        assert!(bank.calculate_and_verify_capitalization(true));

        assert_eq!(
            rent_to_be_distributed,
            bank.rewards
                .read()
                .unwrap()
                .iter()
                .map(|(address, reward)| {
                    if reward.lamports > 0 {
                        assert_eq!(reward.reward_type, RewardType::Rent);
                        if *address == validator_2_pubkey {
                            assert_eq!(reward.post_balance, validator_2_portion + 42 - tweak_2);
                        } else if *address == validator_3_pubkey {
                            assert_eq!(reward.post_balance, validator_3_portion + 42);
                        }
                        reward.lamports as u64
                    } else {
                        0
                    }
                })
                .sum::<u64>()
        );
    }

    #[test]
    fn test_distribute_rent_to_validators_overflow() {
        solana_logger::setup();

        // These values are taken from the real cluster (testnet)
        const RENT_TO_BE_DISTRIBUTED: u64 = 120_525;
        const VALIDATOR_STAKE: u64 = 374_999_998_287_840;

        let validator_pubkey = solana_sdk::pubkey::new_rand();
        let mut genesis_config =
            create_genesis_config_with_leader(10, &validator_pubkey, VALIDATOR_STAKE)
                .genesis_config;

        let bank = Bank::new_for_tests(&genesis_config);
        let old_validator_lamports = bank.get_balance(&validator_pubkey);
        bank.distribute_rent_to_validators(&bank.vote_accounts(), RENT_TO_BE_DISTRIBUTED);
        let new_validator_lamports = bank.get_balance(&validator_pubkey);
        assert_eq!(
            new_validator_lamports,
            old_validator_lamports + RENT_TO_BE_DISTRIBUTED
        );

        genesis_config
            .accounts
            .remove(&feature_set::no_overflow_rent_distribution::id())
            .unwrap();
        let bank = std::panic::AssertUnwindSafe(Bank::new_for_tests(&genesis_config));
        let old_validator_lamports = bank.get_balance(&validator_pubkey);
        let new_validator_lamports = std::panic::catch_unwind(|| {
            bank.distribute_rent_to_validators(&bank.vote_accounts(), RENT_TO_BE_DISTRIBUTED);
            bank.get_balance(&validator_pubkey)
        });

        if let Ok(new_validator_lamports) = new_validator_lamports {
            info!("asserting overflowing incorrect rent distribution");
            assert_ne!(
                new_validator_lamports,
                old_validator_lamports + RENT_TO_BE_DISTRIBUTED
            );
        } else {
            info!("NOT-asserting overflowing incorrect rent distribution");
        }
    }

    #[test]
    fn test_rent_exempt_executable_account() {
        let (mut genesis_config, mint_keypair) = create_genesis_config(100_000);
        genesis_config.rent = rent_with_exemption_threshold(1000.0);

        let root_bank = Arc::new(Bank::new_for_tests(&genesis_config));
        let bank = create_child_bank_for_rent_test(&root_bank, &genesis_config);

        let account_pubkey = solana_sdk::pubkey::new_rand();
        let account_balance = 1;
        let mut account =
            AccountSharedData::new(account_balance, 0, &solana_sdk::pubkey::new_rand());
        account.set_executable(true);
        bank.store_account(&account_pubkey, &account);

        let transfer_lamports = 1;
        let tx = system_transaction::transfer(
            &mint_keypair,
            &account_pubkey,
            transfer_lamports,
            genesis_config.hash(),
        );

        assert_eq!(
            bank.process_transaction(&tx),
            Err(TransactionError::InvalidWritableAccount)
        );
        assert_eq!(bank.get_balance(&account_pubkey), account_balance);
    }

    #[test]
    #[ignore]
    #[allow(clippy::cognitive_complexity)]
    fn test_rent_complex() {
        solana_logger::setup();
        let mock_program_id = Pubkey::new(&[2u8; 32]);

        #[derive(Serialize, Deserialize)]
        enum MockInstruction {
            Deduction,
        }

        fn mock_process_instruction(
            _first_instruction_account: IndexOfAccount,
            invoke_context: &mut InvokeContext,
        ) -> result::Result<(), InstructionError> {
            let transaction_context = &invoke_context.transaction_context;
            let instruction_context = transaction_context.get_current_instruction_context()?;
            let instruction_data = instruction_context.get_instruction_data();
            if let Ok(instruction) = bincode::deserialize(instruction_data) {
                match instruction {
                    MockInstruction::Deduction => {
                        instruction_context
                            .try_borrow_instruction_account(transaction_context, 1)?
                            .checked_add_lamports(1)?;
                        instruction_context
                            .try_borrow_instruction_account(transaction_context, 2)?
                            .checked_sub_lamports(1)?;
                        Ok(())
                    }
                }
            } else {
                Err(InstructionError::InvalidInstructionData)
            }
        }

        let (mut genesis_config, _mint_keypair) = create_genesis_config(10);
        let mut keypairs: Vec<Keypair> = Vec::with_capacity(14);
        for _i in 0..14 {
            keypairs.push(Keypair::new());
        }

        genesis_config.rent = rent_with_exemption_threshold(1000.0);

        let root_bank = Bank::new_for_tests(&genesis_config);
        // until we completely transition to the eager rent collection,
        // we must ensure lazy rent collection doens't get broken!
        root_bank.restore_old_behavior_for_fragile_tests();
        let root_bank = Arc::new(root_bank);
        let mut bank = create_child_bank_for_rent_test(&root_bank, &genesis_config);
        bank.add_builtin("mock_program", &mock_program_id, mock_process_instruction);

        assert_eq!(bank.last_blockhash(), genesis_config.hash());

        let slots_elapsed: u64 = (0..=bank.epoch)
            .map(|epoch| {
                bank.rent_collector
                    .epoch_schedule
                    .get_slots_in_epoch(epoch + 1)
            })
            .sum();
        let generic_rent_due_for_system_account = bank
            .rent_collector
            .rent
            .due(
                bank.get_minimum_balance_for_rent_exemption(0) - 1,
                0,
                slots_elapsed as f64 / bank.rent_collector.slots_per_year,
            )
            .lamports();

        store_accounts_for_rent_test(
            &bank,
            &mut keypairs,
            mock_program_id,
            generic_rent_due_for_system_account,
        );

        let magic_rent_number = 131; // yuck, derive this value programmatically one day

        let t1 = system_transaction::transfer(
            &keypairs[0],
            &keypairs[1].pubkey(),
            1,
            genesis_config.hash(),
        );
        let t2 = system_transaction::transfer(
            &keypairs[2],
            &keypairs[3].pubkey(),
            1,
            genesis_config.hash(),
        );
        let t3 = system_transaction::transfer(
            &keypairs[4],
            &keypairs[5].pubkey(),
            1,
            genesis_config.hash(),
        );
        let t4 = system_transaction::transfer(
            &keypairs[6],
            &keypairs[7].pubkey(),
            generic_rent_due_for_system_account + 1,
            genesis_config.hash(),
        );
        let t5 = system_transaction::transfer(
            &keypairs[8],
            &keypairs[9].pubkey(),
            929,
            genesis_config.hash(),
        );

        let account_metas = vec![
            AccountMeta::new(keypairs[10].pubkey(), true),
            AccountMeta::new(keypairs[11].pubkey(), true),
            AccountMeta::new(keypairs[12].pubkey(), true),
            AccountMeta::new_readonly(keypairs[13].pubkey(), false),
        ];
        let deduct_instruction = Instruction::new_with_bincode(
            mock_program_id,
            &MockInstruction::Deduction,
            account_metas,
        );
        let t6 = Transaction::new_signed_with_payer(
            &[deduct_instruction],
            Some(&keypairs[10].pubkey()),
            &[&keypairs[10], &keypairs[11], &keypairs[12]],
            genesis_config.hash(),
        );

        let txs = vec![t6, t5, t1, t2, t3, t4];
        let res = bank.process_transactions(txs.iter());

        assert_eq!(res.len(), 6);
        assert_eq!(res[0], Ok(()));
        assert_eq!(res[1], Ok(()));
        assert_eq!(res[2], Ok(()));
        assert_eq!(res[3], Ok(()));
        assert_eq!(res[4], Err(TransactionError::AccountNotFound));
        assert_eq!(res[5], Ok(()));

        bank.freeze();

        let mut rent_collected = 0;

        // 48992 - generic_rent_due_for_system_account(Rent) - 1(transfer)
        assert_eq!(bank.get_balance(&keypairs[0].pubkey()), 1);
        rent_collected += generic_rent_due_for_system_account;

        // 48992 - generic_rent_due_for_system_account(Rent) + 1(transfer)
        assert_eq!(bank.get_balance(&keypairs[1].pubkey()), 3);
        rent_collected += generic_rent_due_for_system_account;

        // 48992 - generic_rent_due_for_system_account(Rent) - 1(transfer)
        assert_eq!(bank.get_balance(&keypairs[2].pubkey()), 1);
        rent_collected += generic_rent_due_for_system_account;

        // 48992 - generic_rent_due_for_system_account(Rent) + 1(transfer)
        assert_eq!(bank.get_balance(&keypairs[3].pubkey()), 3);
        rent_collected += generic_rent_due_for_system_account;

        // No rent deducted
        assert_eq!(bank.get_balance(&keypairs[4].pubkey()), 10);
        assert_eq!(bank.get_balance(&keypairs[5].pubkey()), 10);

        // 98004 - generic_rent_due_for_system_account(Rent) - 48991(transfer)
        assert_eq!(bank.get_balance(&keypairs[6].pubkey()), 23);
        rent_collected += generic_rent_due_for_system_account;

        // 0 + 48990(transfer) - magic_rent_number(Rent)
        assert_eq!(
            bank.get_balance(&keypairs[7].pubkey()),
            generic_rent_due_for_system_account + 1 - magic_rent_number
        );

        // Epoch should be updated
        // Rent deducted on store side
        let account8 = bank.get_account(&keypairs[7].pubkey()).unwrap();
        // Epoch should be set correctly.
        assert_eq!(account8.rent_epoch(), bank.epoch + 1);
        rent_collected += magic_rent_number;

        // 49921 - generic_rent_due_for_system_account(Rent) - 929(Transfer)
        assert_eq!(bank.get_balance(&keypairs[8].pubkey()), 2);
        rent_collected += generic_rent_due_for_system_account;

        let account10 = bank.get_account(&keypairs[9].pubkey()).unwrap();
        // Account was overwritten at load time, since it didn't have sufficient balance to pay rent
        // Then, at store time we deducted `magic_rent_number` rent for the current epoch, once it has balance
        assert_eq!(account10.rent_epoch(), bank.epoch + 1);
        // account data is blank now
        assert_eq!(account10.data().len(), 0);
        // 10 - 10(Rent) + 929(Transfer) - magic_rent_number(Rent)
        assert_eq!(account10.lamports(), 929 - magic_rent_number);
        rent_collected += magic_rent_number + 10;

        // 48993 - generic_rent_due_for_system_account(Rent)
        assert_eq!(bank.get_balance(&keypairs[10].pubkey()), 3);
        rent_collected += generic_rent_due_for_system_account;

        // 48993 - generic_rent_due_for_system_account(Rent) + 1(Addition by program)
        assert_eq!(bank.get_balance(&keypairs[11].pubkey()), 4);
        rent_collected += generic_rent_due_for_system_account;

        // 48993 - generic_rent_due_for_system_account(Rent) - 1(Deduction by program)
        assert_eq!(bank.get_balance(&keypairs[12].pubkey()), 2);
        rent_collected += generic_rent_due_for_system_account;

        // No rent for read-only account
        assert_eq!(bank.get_balance(&keypairs[13].pubkey()), 14);

        // Bank's collected rent should be sum of rent collected from all accounts
        assert_eq!(bank.collected_rent.load(Relaxed), rent_collected);
    }

    fn test_rent_collection_partitions(bank: &Bank) -> Vec<Partition> {
        let partitions = bank.rent_collection_partitions();
        let slot = bank.slot();
        if slot.saturating_sub(1) == bank.parent_slot() {
            let partition = Bank::variable_cycle_partition_from_previous_slot(
                bank.epoch_schedule(),
                bank.slot(),
            );
            assert_eq!(
                partitions.last().unwrap(),
                &partition,
                "slot: {}, slots per epoch: {}, partitions: {:?}",
                bank.slot(),
                bank.epoch_schedule().slots_per_epoch,
                partitions
            );
        }
        partitions
    }

    #[test]
    fn test_rent_eager_across_epoch_without_gap() {
        let mut bank = create_simple_test_arc_bank(1);
        assert_eq!(bank.rent_collection_partitions(), vec![(0, 0, 32)]);

        bank = Arc::new(new_from_parent(&bank));
        assert_eq!(bank.rent_collection_partitions(), vec![(0, 1, 32)]);
        for _ in 2..32 {
            bank = Arc::new(new_from_parent(&bank));
        }
        assert_eq!(bank.rent_collection_partitions(), vec![(30, 31, 32)]);
        bank = Arc::new(new_from_parent(&bank));
        assert_eq!(bank.rent_collection_partitions(), vec![(0, 0, 64)]);
    }

    #[test]
    fn test_rent_eager_across_epoch_without_gap_mnb() {
        solana_logger::setup();
        let (mut genesis_config, _mint_keypair) = create_genesis_config(1);
        genesis_config.cluster_type = ClusterType::MainnetBeta;

        let mut bank = Arc::new(Bank::new_for_tests(&genesis_config));
        assert_eq!(test_rent_collection_partitions(&bank), vec![(0, 0, 32)]);

        bank = Arc::new(new_from_parent(&bank));
        assert_eq!(test_rent_collection_partitions(&bank), vec![(0, 1, 32)]);
        for _ in 2..32 {
            bank = Arc::new(new_from_parent(&bank));
        }
        assert_eq!(test_rent_collection_partitions(&bank), vec![(30, 31, 32)]);
        bank = Arc::new(new_from_parent(&bank));
        assert_eq!(test_rent_collection_partitions(&bank), vec![(0, 0, 64)]);
    }

    #[test]
    fn test_rent_eager_across_epoch_with_full_gap() {
        let (mut genesis_config, _mint_keypair) = create_genesis_config(1);
        activate_all_features(&mut genesis_config);

        let mut bank = Arc::new(Bank::new_for_tests(&genesis_config));
        assert_eq!(bank.rent_collection_partitions(), vec![(0, 0, 32)]);

        bank = Arc::new(new_from_parent(&bank));
        assert_eq!(bank.rent_collection_partitions(), vec![(0, 1, 32)]);
        for _ in 2..15 {
            bank = Arc::new(new_from_parent(&bank));
        }
        assert_eq!(bank.rent_collection_partitions(), vec![(13, 14, 32)]);
        bank = Arc::new(Bank::new_from_parent(&bank, &Pubkey::default(), 49));
        assert_eq!(
            bank.rent_collection_partitions(),
            vec![(14, 31, 32), (0, 0, 64), (0, 17, 64)]
        );
        bank = Arc::new(new_from_parent(&bank));
        assert_eq!(bank.rent_collection_partitions(), vec![(17, 18, 64)]);
    }

    #[test]
    fn test_rent_eager_across_epoch_with_half_gap() {
        let (mut genesis_config, _mint_keypair) = create_genesis_config(1);
        activate_all_features(&mut genesis_config);

        let mut bank = Arc::new(Bank::new_for_tests(&genesis_config));
        assert_eq!(bank.rent_collection_partitions(), vec![(0, 0, 32)]);

        bank = Arc::new(new_from_parent(&bank));
        assert_eq!(bank.rent_collection_partitions(), vec![(0, 1, 32)]);
        for _ in 2..15 {
            bank = Arc::new(new_from_parent(&bank));
        }
        assert_eq!(bank.rent_collection_partitions(), vec![(13, 14, 32)]);
        bank = Arc::new(Bank::new_from_parent(&bank, &Pubkey::default(), 32));
        assert_eq!(
            bank.rent_collection_partitions(),
            vec![(14, 31, 32), (0, 0, 64)]
        );
        bank = Arc::new(new_from_parent(&bank));
        assert_eq!(bank.rent_collection_partitions(), vec![(0, 1, 64)]);
    }

    #[test]
    #[allow(clippy::cognitive_complexity)]
    fn test_rent_eager_across_epoch_without_gap_under_multi_epoch_cycle() {
        let leader_pubkey = solana_sdk::pubkey::new_rand();
        let leader_lamports = 3;
        let mut genesis_config =
            create_genesis_config_with_leader(5, &leader_pubkey, leader_lamports).genesis_config;
        genesis_config.cluster_type = ClusterType::MainnetBeta;

        const SLOTS_PER_EPOCH: u64 = MINIMUM_SLOTS_PER_EPOCH as u64;
        const LEADER_SCHEDULE_SLOT_OFFSET: u64 = SLOTS_PER_EPOCH * 3 - 3;
        genesis_config.epoch_schedule =
            EpochSchedule::custom(SLOTS_PER_EPOCH, LEADER_SCHEDULE_SLOT_OFFSET, false);

        let mut bank = Arc::new(Bank::new_for_tests(&genesis_config));
        assert_eq!(DEFAULT_SLOTS_PER_EPOCH, 432_000);
        assert_eq!(bank.get_slots_in_epoch(bank.epoch()), 32);
        assert_eq!(bank.get_epoch_and_slot_index(bank.slot()), (0, 0));
        assert_eq!(bank.rent_collection_partitions(), vec![(0, 0, 432_000)]);

        bank = Arc::new(new_from_parent(&bank));
        assert_eq!(bank.get_slots_in_epoch(bank.epoch()), 32);
        assert_eq!(bank.get_epoch_and_slot_index(bank.slot()), (0, 1));
        assert_eq!(bank.rent_collection_partitions(), vec![(0, 1, 432_000)]);

        for _ in 2..32 {
            bank = Arc::new(new_from_parent(&bank));
        }
        assert_eq!(bank.get_slots_in_epoch(bank.epoch()), 32);
        assert_eq!(bank.get_epoch_and_slot_index(bank.slot()), (0, 31));
        assert_eq!(bank.rent_collection_partitions(), vec![(30, 31, 432_000)]);

        bank = Arc::new(new_from_parent(&bank));
        assert_eq!(bank.get_slots_in_epoch(bank.epoch()), 32);
        assert_eq!(bank.get_epoch_and_slot_index(bank.slot()), (1, 0));
        assert_eq!(bank.rent_collection_partitions(), vec![(31, 32, 432_000)]);

        bank = Arc::new(new_from_parent(&bank));
        assert_eq!(bank.get_slots_in_epoch(bank.epoch()), 32);
        assert_eq!(bank.get_epoch_and_slot_index(bank.slot()), (1, 1));
        assert_eq!(bank.rent_collection_partitions(), vec![(32, 33, 432_000)]);

        bank = Arc::new(Bank::new_from_parent(&bank, &Pubkey::default(), 1000));
        bank = Arc::new(Bank::new_from_parent(&bank, &Pubkey::default(), 1001));
        assert_eq!(bank.get_slots_in_epoch(bank.epoch()), 32);
        assert_eq!(bank.get_epoch_and_slot_index(bank.slot()), (31, 9));
        assert_eq!(
            bank.rent_collection_partitions(),
            vec![(1000, 1001, 432_000)]
        );

        bank = Arc::new(Bank::new_from_parent(&bank, &Pubkey::default(), 431_998));
        bank = Arc::new(Bank::new_from_parent(&bank, &Pubkey::default(), 431_999));
        assert_eq!(bank.get_slots_in_epoch(bank.epoch()), 32);
        assert_eq!(bank.get_epoch_and_slot_index(bank.slot()), (13499, 31));
        assert_eq!(
            bank.rent_collection_partitions(),
            vec![(431_998, 431_999, 432_000)]
        );

        bank = Arc::new(new_from_parent(&bank));
        assert_eq!(bank.get_slots_in_epoch(bank.epoch()), 32);
        assert_eq!(bank.get_epoch_and_slot_index(bank.slot()), (13500, 0));
        assert_eq!(bank.rent_collection_partitions(), vec![(0, 0, 432_000)]);

        bank = Arc::new(new_from_parent(&bank));
        assert_eq!(bank.get_slots_in_epoch(bank.epoch()), 32);
        assert_eq!(bank.get_epoch_and_slot_index(bank.slot()), (13500, 1));
        assert_eq!(bank.rent_collection_partitions(), vec![(0, 1, 432_000)]);
    }

    #[test]
    fn test_rent_eager_across_epoch_with_gap_under_multi_epoch_cycle() {
        let leader_pubkey = solana_sdk::pubkey::new_rand();
        let leader_lamports = 3;
        let mut genesis_config =
            create_genesis_config_with_leader(5, &leader_pubkey, leader_lamports).genesis_config;
        genesis_config.cluster_type = ClusterType::MainnetBeta;

        const SLOTS_PER_EPOCH: u64 = MINIMUM_SLOTS_PER_EPOCH as u64;
        const LEADER_SCHEDULE_SLOT_OFFSET: u64 = SLOTS_PER_EPOCH * 3 - 3;
        genesis_config.epoch_schedule =
            EpochSchedule::custom(SLOTS_PER_EPOCH, LEADER_SCHEDULE_SLOT_OFFSET, false);

        let mut bank = Arc::new(Bank::new_for_tests(&genesis_config));
        assert_eq!(DEFAULT_SLOTS_PER_EPOCH, 432_000);
        assert_eq!(bank.get_slots_in_epoch(bank.epoch()), 32);
        assert_eq!(bank.get_epoch_and_slot_index(bank.slot()), (0, 0));
        assert_eq!(bank.rent_collection_partitions(), vec![(0, 0, 432_000)]);

        bank = Arc::new(new_from_parent(&bank));
        assert_eq!(bank.get_slots_in_epoch(bank.epoch()), 32);
        assert_eq!(bank.get_epoch_and_slot_index(bank.slot()), (0, 1));
        assert_eq!(bank.rent_collection_partitions(), vec![(0, 1, 432_000)]);

        for _ in 2..19 {
            bank = Arc::new(new_from_parent(&bank));
        }
        assert_eq!(bank.get_slots_in_epoch(bank.epoch()), 32);
        assert_eq!(bank.get_epoch_and_slot_index(bank.slot()), (0, 18));
        assert_eq!(bank.rent_collection_partitions(), vec![(17, 18, 432_000)]);

        bank = Arc::new(Bank::new_from_parent(&bank, &Pubkey::default(), 44));
        assert_eq!(bank.get_slots_in_epoch(bank.epoch()), 32);
        assert_eq!(bank.get_epoch_and_slot_index(bank.slot()), (1, 12));
        assert_eq!(
            bank.rent_collection_partitions(),
            vec![(18, 31, 432_000), (31, 31, 432_000), (31, 44, 432_000)]
        );

        bank = Arc::new(new_from_parent(&bank));
        assert_eq!(bank.get_slots_in_epoch(bank.epoch()), 32);
        assert_eq!(bank.get_epoch_and_slot_index(bank.slot()), (1, 13));
        assert_eq!(bank.rent_collection_partitions(), vec![(44, 45, 432_000)]);

        bank = Arc::new(Bank::new_from_parent(&bank, &Pubkey::default(), 431_993));
        bank = Arc::new(Bank::new_from_parent(&bank, &Pubkey::default(), 432_011));
        assert_eq!(bank.get_slots_in_epoch(bank.epoch()), 32);
        assert_eq!(bank.get_epoch_and_slot_index(bank.slot()), (13500, 11));
        assert_eq!(
            bank.rent_collection_partitions(),
            vec![
                (431_993, 431_999, 432_000),
                (0, 0, 432_000),
                (0, 11, 432_000)
            ]
        );
    }

    #[test]
    fn test_rent_eager_with_warmup_epochs_under_multi_epoch_cycle() {
        let leader_pubkey = solana_sdk::pubkey::new_rand();
        let leader_lamports = 3;
        let mut genesis_config =
            create_genesis_config_with_leader(5, &leader_pubkey, leader_lamports).genesis_config;
        genesis_config.cluster_type = ClusterType::MainnetBeta;

        const SLOTS_PER_EPOCH: u64 = MINIMUM_SLOTS_PER_EPOCH as u64 * 8;
        const LEADER_SCHEDULE_SLOT_OFFSET: u64 = SLOTS_PER_EPOCH * 3 - 3;
        genesis_config.epoch_schedule =
            EpochSchedule::custom(SLOTS_PER_EPOCH, LEADER_SCHEDULE_SLOT_OFFSET, true);

        let mut bank = Arc::new(Bank::new_for_tests(&genesis_config));
        assert_eq!(DEFAULT_SLOTS_PER_EPOCH, 432_000);
        assert_eq!(bank.get_slots_in_epoch(bank.epoch()), 32);
        assert_eq!(bank.first_normal_epoch(), 3);
        assert_eq!(bank.get_epoch_and_slot_index(bank.slot()), (0, 0));
        assert_eq!(bank.rent_collection_partitions(), vec![(0, 0, 32)]);

        bank = Arc::new(Bank::new_from_parent(&bank, &Pubkey::default(), 222));
        bank = Arc::new(new_from_parent(&bank));
        assert_eq!(bank.get_slots_in_epoch(bank.epoch()), 128);
        assert_eq!(bank.get_epoch_and_slot_index(bank.slot()), (2, 127));
        assert_eq!(bank.rent_collection_partitions(), vec![(126, 127, 128)]);

        bank = Arc::new(new_from_parent(&bank));
        assert_eq!(bank.get_slots_in_epoch(bank.epoch()), 256);
        assert_eq!(bank.get_epoch_and_slot_index(bank.slot()), (3, 0));
        assert_eq!(bank.rent_collection_partitions(), vec![(0, 0, 431_872)]);
        assert_eq!(431_872 % bank.get_slots_in_epoch(bank.epoch()), 0);

        bank = Arc::new(new_from_parent(&bank));
        assert_eq!(bank.get_slots_in_epoch(bank.epoch()), 256);
        assert_eq!(bank.get_epoch_and_slot_index(bank.slot()), (3, 1));
        assert_eq!(bank.rent_collection_partitions(), vec![(0, 1, 431_872)]);

        bank = Arc::new(Bank::new_from_parent(
            &bank,
            &Pubkey::default(),
            431_872 + 223 - 1,
        ));
        bank = Arc::new(new_from_parent(&bank));
        assert_eq!(bank.get_slots_in_epoch(bank.epoch()), 256);
        assert_eq!(bank.get_epoch_and_slot_index(bank.slot()), (1689, 255));
        assert_eq!(
            bank.rent_collection_partitions(),
            vec![(431_870, 431_871, 431_872)]
        );

        bank = Arc::new(new_from_parent(&bank));
        assert_eq!(bank.get_slots_in_epoch(bank.epoch()), 256);
        assert_eq!(bank.get_epoch_and_slot_index(bank.slot()), (1690, 0));
        assert_eq!(bank.rent_collection_partitions(), vec![(0, 0, 431_872)]);
    }

    #[test]
    fn test_rent_eager_under_fixed_cycle_for_development() {
        solana_logger::setup();
        let leader_pubkey = solana_sdk::pubkey::new_rand();
        let leader_lamports = 3;
        let mut genesis_config =
            create_genesis_config_with_leader(5, &leader_pubkey, leader_lamports).genesis_config;

        const SLOTS_PER_EPOCH: u64 = MINIMUM_SLOTS_PER_EPOCH as u64 * 8;
        const LEADER_SCHEDULE_SLOT_OFFSET: u64 = SLOTS_PER_EPOCH * 3 - 3;
        genesis_config.epoch_schedule =
            EpochSchedule::custom(SLOTS_PER_EPOCH, LEADER_SCHEDULE_SLOT_OFFSET, true);

        let mut bank = Arc::new(Bank::new_for_tests(&genesis_config));
        assert_eq!(bank.get_slots_in_epoch(bank.epoch()), 32);
        assert_eq!(bank.first_normal_epoch(), 3);
        assert_eq!(bank.get_epoch_and_slot_index(bank.slot()), (0, 0));
        assert_eq!(bank.rent_collection_partitions(), vec![(0, 0, 432_000)]);

        bank = Arc::new(Bank::new_from_parent(&bank, &Pubkey::default(), 222));
        bank = Arc::new(new_from_parent(&bank));
        assert_eq!(bank.get_slots_in_epoch(bank.epoch()), 128);
        assert_eq!(bank.get_epoch_and_slot_index(bank.slot()), (2, 127));
        assert_eq!(bank.rent_collection_partitions(), vec![(222, 223, 432_000)]);

        bank = Arc::new(new_from_parent(&bank));
        assert_eq!(bank.get_slots_in_epoch(bank.epoch()), 256);
        assert_eq!(bank.get_epoch_and_slot_index(bank.slot()), (3, 0));
        assert_eq!(bank.rent_collection_partitions(), vec![(223, 224, 432_000)]);

        bank = Arc::new(new_from_parent(&bank));
        assert_eq!(bank.get_slots_in_epoch(bank.epoch()), 256);
        assert_eq!(bank.get_epoch_and_slot_index(bank.slot()), (3, 1));
        assert_eq!(bank.rent_collection_partitions(), vec![(224, 225, 432_000)]);

        bank = Arc::new(Bank::new_from_parent(
            &bank,
            &Pubkey::default(),
            432_000 - 2,
        ));
        bank = Arc::new(new_from_parent(&bank));
        assert_eq!(
            bank.rent_collection_partitions(),
            vec![(431_998, 431_999, 432_000)]
        );
        bank = Arc::new(new_from_parent(&bank));
        assert_eq!(bank.rent_collection_partitions(), vec![(0, 0, 432_000)]);
        bank = Arc::new(new_from_parent(&bank));
        assert_eq!(bank.rent_collection_partitions(), vec![(0, 1, 432_000)]);

        bank = Arc::new(Bank::new_from_parent(
            &bank,
            &Pubkey::default(),
            864_000 - 20,
        ));
        bank = Arc::new(Bank::new_from_parent(
            &bank,
            &Pubkey::default(),
            864_000 + 39,
        ));
        assert_eq!(
            bank.rent_collection_partitions(),
            vec![
                (431_980, 431_999, 432_000),
                (0, 0, 432_000),
                (0, 39, 432_000)
            ]
        );
    }

    #[test]
    fn test_rent_eager_pubkey_range_minimal() {
        let range = Bank::pubkey_range_from_partition((0, 0, 1));
        assert_eq!(
            range,
            Pubkey::new_from_array([0x00; 32])..=Pubkey::new_from_array([0xff; 32])
        );
    }

    #[test]
    fn test_rent_eager_pubkey_range_maximum() {
        let max = !0;

        let range = Bank::pubkey_range_from_partition((0, 0, max));
        assert_eq!(
            range,
            Pubkey::new_from_array([0x00; 32])
                ..=Pubkey::new_from_array([
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 0xff,
                    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                    0xff, 0xff, 0xff, 0xff, 0xff, 0xff
                ])
        );
        let range = Bank::pubkey_range_from_partition((0, 1, max));
        const ONE: u8 = 0x01;
        assert_eq!(
            range,
            Pubkey::new_from_array([
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, ONE, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00,
            ])
                ..=Pubkey::new_from_array([
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0xff, 0xff, 0xff, 0xff, 0xff,
                    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                    0xff, 0xff, 0xff, 0xff, 0xff, 0xff
                ])
        );
        let range = Bank::pubkey_range_from_partition((max - 3, max - 2, max));
        const FD: u8 = 0xfd;
        assert_eq!(
            range,
            Pubkey::new_from_array([
                0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xfd, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00,
            ])
                ..=Pubkey::new_from_array([
                    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, FD, 0xff, 0xff, 0xff, 0xff, 0xff,
                    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                    0xff, 0xff, 0xff, 0xff, 0xff, 0xff
                ])
        );
        let range = Bank::pubkey_range_from_partition((max - 2, max - 1, max));
        assert_eq!(
            range,
            Pubkey::new_from_array([
                0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xfe, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00,
            ])..=pubkey_max_value()
        );

        fn should_cause_overflow(partition_count: u64) -> bool {
            // Check `partition_width = (u64::max_value() + 1) / partition_count` is exact and
            // does not have a remainder.
            // This way, `partition_width * partition_count == (u64::max_value() + 1)`,
            // so the test actually tests for overflow
            (u64::max_value() - partition_count + 1) % partition_count == 0
        }

        let max_exact = 64;
        // Make sure `max_exact` divides evenly when calculating `calculate_partition_width`
        assert!(should_cause_overflow(max_exact));
        // Make sure `max_inexact` doesn't divide evenly when calculating `calculate_partition_width`
        let max_inexact = 10;
        assert!(!should_cause_overflow(max_inexact));

        for max in &[max_exact, max_inexact] {
            let range = Bank::pubkey_range_from_partition((max - 1, max - 1, *max));
            assert_eq!(range, pubkey_max_value()..=pubkey_max_value());
        }
    }

    fn map_to_test_bad_range() -> std::collections::BTreeMap<Pubkey, i8> {
        let mut map = std::collections::BTreeMap::new();
        // when empty, std::collections::BTreeMap doesn't sanitize given range...
        map.insert(solana_sdk::pubkey::new_rand(), 1);
        map
    }

    #[test]
    #[should_panic(expected = "range start is greater than range end in BTreeMap")]
    fn test_rent_eager_bad_range() {
        let test_map = map_to_test_bad_range();
        let _ = test_map.range(
            Pubkey::new_from_array([
                0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x01,
            ])
                ..=Pubkey::new_from_array([
                    0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                ]),
        );
    }

    #[test]
    fn test_rent_eager_pubkey_range_noop_range() {
        let test_map = map_to_test_bad_range();

        let range = Bank::pubkey_range_from_partition((0, 0, 3));
        assert_eq!(
            range,
            Pubkey::new_from_array([0x00; 32])
                ..=Pubkey::new_from_array([
                    0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x54, 0xff, 0xff, 0xff, 0xff, 0xff,
                    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                    0xff, 0xff, 0xff, 0xff, 0xff, 0xff
                ])
        );
        let _ = test_map.range(range);

        let range = Bank::pubkey_range_from_partition((1, 1, 3));
        let same = Pubkey::new_from_array([
            0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ]);
        assert_eq!(range, same..=same);
        let _ = test_map.range(range);

        let range = Bank::pubkey_range_from_partition((2, 2, 3));
        assert_eq!(range, pubkey_max_value()..=pubkey_max_value());
        let _ = test_map.range(range);
    }

    fn pubkey_max_value() -> Pubkey {
        let highest = Pubkey::from_str("JEKNVnkbo3jma5nREBBJCDoXFVeKkD56V3xKrvRmWxFG").unwrap();
        let arr = Pubkey::new_from_array([0xff; 32]);
        assert_eq!(highest, arr);
        arr
    }

    #[test]
    fn test_rent_pubkey_range_max() {
        // start==end && start != 0 is curious behavior. Verifying it here.
        solana_logger::setup();
        let range = Bank::pubkey_range_from_partition((1, 1, 3));
        let p = Bank::partition_from_pubkey(range.start(), 3);
        assert_eq!(p, 2);
        let range = Bank::pubkey_range_from_partition((1, 2, 3));
        let p = Bank::partition_from_pubkey(range.start(), 3);
        assert_eq!(p, 2);
        let range = Bank::pubkey_range_from_partition((2, 2, 3));
        let p = Bank::partition_from_pubkey(range.start(), 3);
        assert_eq!(p, 2);
        let range = Bank::pubkey_range_from_partition((1, 1, 16));
        let p = Bank::partition_from_pubkey(range.start(), 16);
        assert_eq!(p, 2);
        let range = Bank::pubkey_range_from_partition((1, 2, 16));
        let p = Bank::partition_from_pubkey(range.start(), 16);
        assert_eq!(p, 2);
        let range = Bank::pubkey_range_from_partition((2, 2, 16));
        let p = Bank::partition_from_pubkey(range.start(), 16);
        assert_eq!(p, 3);
        let range = Bank::pubkey_range_from_partition((15, 15, 16));
        let p = Bank::partition_from_pubkey(range.start(), 16);
        assert_eq!(p, 15);
    }

    #[test]
    fn test_rent_eager_pubkey_range_dividable() {
        let test_map = map_to_test_bad_range();
        let range = Bank::pubkey_range_from_partition((0, 0, 2));

        assert_eq!(
            range,
            Pubkey::new_from_array([0x00; 32])
                ..=Pubkey::new_from_array([
                    0x7f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                    0xff, 0xff, 0xff, 0xff, 0xff, 0xff
                ])
        );
        let _ = test_map.range(range);

        let range = Bank::pubkey_range_from_partition((0, 1, 2));
        assert_eq!(
            range,
            Pubkey::new_from_array([
                0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00
            ])
                ..=Pubkey::new_from_array([
                    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                    0xff, 0xff, 0xff, 0xff, 0xff, 0xff
                ])
        );
        let _ = test_map.range(range);
    }

    #[test]
    fn test_rent_eager_pubkey_range_not_dividable() {
        solana_logger::setup();

        let test_map = map_to_test_bad_range();
        let range = Bank::pubkey_range_from_partition((0, 0, 3));
        assert_eq!(
            range,
            Pubkey::new_from_array([0x00; 32])
                ..=Pubkey::new_from_array([
                    0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x54, 0xff, 0xff, 0xff, 0xff, 0xff,
                    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                    0xff, 0xff, 0xff, 0xff, 0xff, 0xff
                ])
        );
        let _ = test_map.range(range);

        let range = Bank::pubkey_range_from_partition((0, 1, 3));
        assert_eq!(
            range,
            Pubkey::new_from_array([
                0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00
            ])
                ..=Pubkey::new_from_array([
                    0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xa9, 0xff, 0xff, 0xff, 0xff, 0xff,
                    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                    0xff, 0xff, 0xff, 0xff, 0xff, 0xff
                ])
        );
        let _ = test_map.range(range);

        let range = Bank::pubkey_range_from_partition((1, 2, 3));
        assert_eq!(
            range,
            Pubkey::new_from_array([
                0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00
            ])
                ..=Pubkey::new_from_array([
                    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                    0xff, 0xff, 0xff, 0xff, 0xff, 0xff
                ])
        );
        let _ = test_map.range(range);
    }

    #[test]
    fn test_rent_eager_pubkey_range_gap() {
        solana_logger::setup();

        let test_map = map_to_test_bad_range();
        let range = Bank::pubkey_range_from_partition((120, 1023, 12345));
        assert_eq!(
            range,
            Pubkey::new_from_array([
                0x02, 0x82, 0x5a, 0x89, 0xd1, 0xac, 0x58, 0x9c, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00
            ])
                ..=Pubkey::new_from_array([
                    0x15, 0x3c, 0x1d, 0xf1, 0xc6, 0x39, 0xef, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                    0xff, 0xff, 0xff, 0xff, 0xff, 0xff
                ])
        );
        let _ = test_map.range(range);
    }

    impl Bank {
        fn slots_by_pubkey(&self, pubkey: &Pubkey, ancestors: &Ancestors) -> Vec<Slot> {
            let (locked_entry, _) = self
                .rc
                .accounts
                .accounts_db
                .accounts_index
                .get(pubkey, Some(ancestors), None)
                .unwrap();
            locked_entry
                .slot_list()
                .iter()
                .map(|(slot, _)| *slot)
                .collect::<Vec<Slot>>()
        }
    }

    #[test]
    fn test_rent_eager_collect_rent_in_partition() {
        solana_logger::setup();

        let (mut genesis_config, _mint_keypair) = create_genesis_config(1_000_000);
        activate_all_features(&mut genesis_config);

        let zero_lamport_pubkey = solana_sdk::pubkey::new_rand();
        let rent_due_pubkey = solana_sdk::pubkey::new_rand();
        let rent_exempt_pubkey = solana_sdk::pubkey::new_rand();

        let mut bank = Arc::new(Bank::new_for_tests(&genesis_config));
        let zero_lamports = 0;
        let little_lamports = 1234;
        let large_lamports = 123_456_789;
        // genesis_config.epoch_schedule.slots_per_epoch == 432_000 and is unsuitable for this test
        let some_slot = MINIMUM_SLOTS_PER_EPOCH; // chosen to cause epoch to be +1
        let rent_collected = 1; // this is a function of 'some_slot'

        bank.store_account(
            &zero_lamport_pubkey,
            &AccountSharedData::new(zero_lamports, 0, &Pubkey::default()),
        );
        bank.store_account(
            &rent_due_pubkey,
            &AccountSharedData::new(little_lamports, 0, &Pubkey::default()),
        );
        bank.store_account(
            &rent_exempt_pubkey,
            &AccountSharedData::new(large_lamports, 0, &Pubkey::default()),
        );

        let genesis_slot = 0;
        let ancestors = vec![(some_slot, 0), (0, 1)].into_iter().collect();

        let previous_epoch = bank.epoch();
        bank = Arc::new(Bank::new_from_parent(&bank, &Pubkey::default(), some_slot));
        let current_epoch = bank.epoch();
        assert_eq!(previous_epoch + 1, current_epoch);

        assert_eq!(bank.collected_rent.load(Relaxed), 0);
        assert_eq!(
            bank.get_account(&rent_due_pubkey).unwrap().lamports(),
            little_lamports
        );
        assert_eq!(bank.get_account(&rent_due_pubkey).unwrap().rent_epoch(), 0);
        assert_eq!(
            bank.slots_by_pubkey(&rent_due_pubkey, &ancestors),
            vec![genesis_slot]
        );
        assert_eq!(
            bank.slots_by_pubkey(&rent_exempt_pubkey, &ancestors),
            vec![genesis_slot]
        );
        assert_eq!(
            bank.slots_by_pubkey(&zero_lamport_pubkey, &ancestors),
            vec![genesis_slot]
        );

        assert_eq!(bank.collected_rent.load(Relaxed), 0);
        assert!(bank.rewrites_skipped_this_slot.read().unwrap().is_empty());
        bank.collect_rent_in_partition((0, 0, 1), true, &RentMetrics::default());
        {
            let rewrites_skipped = bank.rewrites_skipped_this_slot.read().unwrap();
            // `rewrites_skipped.len()` is the number of non-rent paying accounts in the slot.
            // 'collect_rent_in_partition' fills 'rewrites_skipped_this_slot' with rewrites that
            // were skipped during rent collection but should still be considered in the slot's
            // bank hash. If the slot is also written in the append vec, then the bank hash calc
            // code ignores the contents of this list. This assert is confirming that the expected #
            // of accounts were included in 'rewrites_skipped' by the call to
            // 'collect_rent_in_partition(..., true)' above.
            assert_eq!(rewrites_skipped.len(), 1);
            // should not have skipped 'rent_exempt_pubkey'
            // Once preserve_rent_epoch_for_rent_exempt_accounts is activated,
            // rewrite-skip is irrelevant to rent-exempt accounts.
            assert!(!rewrites_skipped.contains_key(&rent_exempt_pubkey));
            // should NOT have skipped 'rent_due_pubkey'
            assert!(!rewrites_skipped.contains_key(&rent_due_pubkey));
        }

        assert_eq!(bank.collected_rent.load(Relaxed), 0);
        bank.collect_rent_in_partition((0, 0, 1), false, &RentMetrics::default()); // all range

        assert_eq!(bank.collected_rent.load(Relaxed), rent_collected);
        assert_eq!(
            bank.get_account(&rent_due_pubkey).unwrap().lamports(),
            little_lamports - rent_collected
        );
        assert_eq!(
            bank.get_account(&rent_due_pubkey).unwrap().rent_epoch(),
            current_epoch + 1
        );
        assert_eq!(
            bank.get_account(&rent_exempt_pubkey).unwrap().lamports(),
            large_lamports
        );
        // Once preserve_rent_epoch_for_rent_exempt_accounts is activated,
        // rent_epoch of rent-exempt accounts will no longer advance.
        assert_eq!(
            bank.get_account(&rent_exempt_pubkey).unwrap().rent_epoch(),
            0
        );
        assert_eq!(
            bank.slots_by_pubkey(&rent_due_pubkey, &ancestors),
            vec![genesis_slot, some_slot]
        );
        assert_eq!(
            bank.slots_by_pubkey(&rent_exempt_pubkey, &ancestors),
            vec![genesis_slot, some_slot]
        );
        assert_eq!(
            bank.slots_by_pubkey(&zero_lamport_pubkey, &ancestors),
            vec![genesis_slot]
        );
    }

    fn new_from_parent_next_epoch(parent: &Arc<Bank>, epochs: Epoch) -> Bank {
        let mut slot = parent.slot();
        let mut epoch = parent.epoch();
        for _ in 0..epochs {
            slot += parent.epoch_schedule().get_slots_in_epoch(epoch);
            epoch = parent.epoch_schedule().get_epoch(slot);
        }

        Bank::new_from_parent(parent, &Pubkey::default(), slot)
    }

    #[test]
    /// tests that an account which has already had rent collected IN this slot does not skip rewrites
    fn test_collect_rent_from_accounts() {
        solana_logger::setup();

        let zero_lamport_pubkey = Pubkey::new(&[0; 32]);

        let genesis_bank = create_simple_test_arc_bank(100000);
        let first_bank = Arc::new(new_from_parent(&genesis_bank));
        let first_slot = 1;
        assert_eq!(first_slot, first_bank.slot());
        let epoch_delta = 4;
        let later_bank = Arc::new(new_from_parent_next_epoch(&first_bank, epoch_delta)); // a bank a few epochs in the future
        let later_slot = later_bank.slot();
        assert!(later_bank.epoch() == genesis_bank.epoch() + epoch_delta);

        let data_size = 0; // make sure we're rent exempt
        let lamports = later_bank.get_minimum_balance_for_rent_exemption(data_size); // cannot be 0 or we zero out rent_epoch in rent collection and we need to be rent exempt
        let mut account = AccountSharedData::new(lamports, data_size, &Pubkey::default());
        account.set_rent_epoch(later_bank.epoch() - 1); // non-zero, but less than later_bank's epoch

        let just_rewrites = true;
        // 'later_slot' here is the slot the account was loaded from.
        // Since 'later_slot' is the same slot the bank is in, this means that the account was already written IN this slot.
        // So, we should NOT skip rewrites.
        let result = later_bank.collect_rent_from_accounts(
            vec![(zero_lamport_pubkey, account.clone(), later_slot)],
            just_rewrites,
            None,
            PartitionIndex::default(),
        );
        assert!(result.rewrites_skipped.is_empty());
        // loaded from previous slot, so we skip rent collection on it
        let result = later_bank.collect_rent_from_accounts(
            vec![(zero_lamport_pubkey, account, later_slot - 1)],
            just_rewrites,
            None,
            PartitionIndex::default(),
        );
        assert!(result.rewrites_skipped[0].0 == zero_lamport_pubkey);
    }

    #[test]
    fn test_rent_eager_collect_rent_zero_lamport_deterministic() {
        solana_logger::setup();

        let (genesis_config, _mint_keypair) = create_genesis_config(1);

        let zero_lamport_pubkey = solana_sdk::pubkey::new_rand();

        let genesis_bank1 = Arc::new(Bank::new_for_tests(&genesis_config));
        let genesis_bank2 = Arc::new(Bank::new_for_tests(&genesis_config));
        let bank1_with_zero = Arc::new(new_from_parent(&genesis_bank1));
        let bank1_without_zero = Arc::new(new_from_parent(&genesis_bank2));

        let zero_lamports = 0;
        let data_size = 12345; // use non-zero data size to also test accounts_data_size
        let account = AccountSharedData::new(zero_lamports, data_size, &Pubkey::default());
        bank1_with_zero.store_account(&zero_lamport_pubkey, &account);
        bank1_without_zero.store_account(&zero_lamport_pubkey, &account);

        bank1_without_zero
            .rc
            .accounts
            .accounts_db
            .accounts_index
            .add_root(genesis_bank1.slot() + 1, false);
        bank1_without_zero
            .rc
            .accounts
            .accounts_db
            .accounts_index
            .purge_roots(&zero_lamport_pubkey);

        // genesis_config.epoch_schedule.slots_per_epoch == 432_000 and is unsuitable for this test
        let some_slot = MINIMUM_SLOTS_PER_EPOCH; // 1 epoch
        let bank2_with_zero = Arc::new(Bank::new_from_parent(
            &bank1_with_zero,
            &Pubkey::default(),
            some_slot,
        ));
        assert_eq!(bank1_with_zero.epoch() + 1, bank2_with_zero.epoch());
        let bank2_without_zero = Arc::new(Bank::new_from_parent(
            &bank1_without_zero,
            &Pubkey::default(),
            some_slot,
        ));
        let hash1_with_zero = bank1_with_zero.hash();
        let hash1_without_zero = bank1_without_zero.hash();
        assert_eq!(hash1_with_zero, hash1_without_zero);
        assert_ne!(hash1_with_zero, Hash::default());

        bank2_with_zero.collect_rent_in_partition((0, 0, 1), false, &RentMetrics::default()); // all
        bank2_without_zero.collect_rent_in_partition((0, 0, 1), false, &RentMetrics::default()); // all

        bank2_with_zero.freeze();
        let hash2_with_zero = bank2_with_zero.hash();
        bank2_without_zero.freeze();
        let hash2_without_zero = bank2_without_zero.hash();

        assert_eq!(hash2_with_zero, hash2_without_zero);
        assert_ne!(hash2_with_zero, Hash::default());
    }

    #[test]
    fn test_bank_update_vote_stake_rewards() {
        let thread_pool = ThreadPoolBuilder::new().num_threads(1).build().unwrap();
        check_bank_update_vote_stake_rewards(|bank: &Bank| {
            bank.load_vote_and_stake_accounts_with_thread_pool(&thread_pool, null_tracer())
        });
        check_bank_update_vote_stake_rewards(|bank: &Bank| {
            bank.load_vote_and_stake_accounts(&thread_pool, null_tracer())
        });
    }

    fn check_bank_update_vote_stake_rewards<F>(load_vote_and_stake_accounts: F)
    where
        F: Fn(&Bank) -> LoadVoteAndStakeAccountsResult,
    {
        solana_logger::setup();

        // create a bank that ticks really slowly...
        let bank0 = Arc::new(Bank::new_for_tests(&GenesisConfig {
            accounts: (0..42)
                .map(|_| {
                    (
                        solana_sdk::pubkey::new_rand(),
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
            cluster_type: ClusterType::MainnetBeta,

            ..GenesisConfig::default()
        }));

        // enable lazy rent collection because this test depends on rent-due accounts
        // not being eagerly-collected for exact rewards calculation
        bank0.restore_old_behavior_for_fragile_tests();

        assert_eq!(
            bank0.capitalization(),
            42 * 1_000_000_000 + genesis_sysvar_and_builtin_program_lamports(),
        );

        let ((vote_id, mut vote_account), (stake_id, stake_account)) =
            crate::stakes::tests::create_staked_node_accounts(10_000);
        let starting_vote_and_stake_balance = 10_000 + 1;

        // set up accounts
        bank0.store_account_and_update_capitalization(&stake_id, &stake_account);

        // generate some rewards
        let mut vote_state = Some(vote_state::from(&vote_account).unwrap());
        for i in 0..MAX_LOCKOUT_HISTORY + 42 {
            if let Some(v) = vote_state.as_mut() {
                vote_state::process_slot_vote_unchecked(v, i as u64)
            }
            let versioned = VoteStateVersions::Current(Box::new(vote_state.take().unwrap()));
            vote_state::to(&versioned, &mut vote_account).unwrap();
            bank0.store_account_and_update_capitalization(&vote_id, &vote_account);
            match versioned {
                VoteStateVersions::Current(v) => {
                    vote_state = Some(*v);
                }
                _ => panic!("Has to be of type Current"),
            };
        }
        bank0.store_account_and_update_capitalization(&vote_id, &vote_account);
        bank0.freeze();

        assert_eq!(
            bank0.capitalization(),
            42 * 1_000_000_000
                + genesis_sysvar_and_builtin_program_lamports()
                + starting_vote_and_stake_balance
                + bank0_sysvar_delta(),
        );
        assert!(bank0.rewards.read().unwrap().is_empty());

        load_vote_and_stake_accounts(&bank0);

        // put a child bank in epoch 1, which calls update_rewards()...
        let bank1 = Bank::new_from_parent(
            &bank0,
            &Pubkey::default(),
            bank0.get_slots_in_epoch(bank0.epoch()) + 1,
        );
        // verify that there's inflation
        assert_ne!(bank1.capitalization(), bank0.capitalization());

        // verify the inflation is represented in validator_points
        let paid_rewards = bank1.capitalization() - bank0.capitalization() - bank1_sysvar_delta();

        // this assumes that no new builtins or precompiles were activated in bank1
        let PrevEpochInflationRewards {
            validator_rewards, ..
        } = bank1.calculate_previous_epoch_inflation_rewards(bank0.capitalization(), bank0.epoch());

        // verify the stake and vote accounts are the right size
        assert!(
            ((bank1.get_balance(&stake_id) - stake_account.lamports() + bank1.get_balance(&vote_id)
                - vote_account.lamports()) as f64
                - validator_rewards as f64)
                .abs()
                < 1.0
        );

        // verify the rewards are the right size
        assert!((validator_rewards as f64 - paid_rewards as f64).abs() < 1.0); // rounding, truncating

        // verify validator rewards show up in bank1.rewards vector
        assert_eq!(
            *bank1.rewards.read().unwrap(),
            vec![(
                stake_id,
                RewardInfo {
                    reward_type: RewardType::Staking,
                    lamports: validator_rewards as i64,
                    post_balance: bank1.get_balance(&stake_id),
                    commission: Some(0),
                }
            )]
        );
        bank1.freeze();
        assert!(bank1.calculate_and_verify_capitalization(true));
    }

    fn do_test_bank_update_rewards_determinism() -> u64 {
        // create a bank that ticks really slowly...
        let bank = Arc::new(Bank::new_for_tests(&GenesisConfig {
            accounts: (0..42)
                .map(|_| {
                    (
                        solana_sdk::pubkey::new_rand(),
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
            cluster_type: ClusterType::MainnetBeta,

            ..GenesisConfig::default()
        }));

        // enable lazy rent collection because this test depends on rent-due accounts
        // not being eagerly-collected for exact rewards calculation
        bank.restore_old_behavior_for_fragile_tests();

        assert_eq!(
            bank.capitalization(),
            42 * 1_000_000_000 + genesis_sysvar_and_builtin_program_lamports()
        );

        let vote_id = solana_sdk::pubkey::new_rand();
        let mut vote_account =
            vote_state::create_account(&vote_id, &solana_sdk::pubkey::new_rand(), 50, 100);
        let (stake_id1, stake_account1) = crate::stakes::tests::create_stake_account(123, &vote_id);
        let (stake_id2, stake_account2) = crate::stakes::tests::create_stake_account(456, &vote_id);

        // set up accounts
        bank.store_account_and_update_capitalization(&stake_id1, &stake_account1);
        bank.store_account_and_update_capitalization(&stake_id2, &stake_account2);

        // generate some rewards
        let mut vote_state = Some(vote_state::from(&vote_account).unwrap());
        for i in 0..MAX_LOCKOUT_HISTORY + 42 {
            if let Some(v) = vote_state.as_mut() {
                vote_state::process_slot_vote_unchecked(v, i as u64)
            }
            let versioned = VoteStateVersions::Current(Box::new(vote_state.take().unwrap()));
            vote_state::to(&versioned, &mut vote_account).unwrap();
            bank.store_account_and_update_capitalization(&vote_id, &vote_account);
            match versioned {
                VoteStateVersions::Current(v) => {
                    vote_state = Some(*v);
                }
                _ => panic!("Has to be of type Current"),
            };
        }
        bank.store_account_and_update_capitalization(&vote_id, &vote_account);

        // put a child bank in epoch 1, which calls update_rewards()...
        let bank1 = Bank::new_from_parent(
            &bank,
            &Pubkey::default(),
            bank.get_slots_in_epoch(bank.epoch()) + 1,
        );
        // verify that there's inflation
        assert_ne!(bank1.capitalization(), bank.capitalization());

        bank1.freeze();
        assert!(bank1.calculate_and_verify_capitalization(true));

        // verify voting and staking rewards are recorded
        let rewards = bank1.rewards.read().unwrap();
        rewards
            .iter()
            .find(|(_address, reward)| reward.reward_type == RewardType::Voting)
            .unwrap();
        rewards
            .iter()
            .find(|(_address, reward)| reward.reward_type == RewardType::Staking)
            .unwrap();

        bank1.capitalization()
    }

    #[test]
    fn test_bank_update_rewards_determinism() {
        solana_logger::setup();

        // The same reward should be distributed given same credits
        let expected_capitalization = do_test_bank_update_rewards_determinism();
        // Repeat somewhat large number of iterations to expose possible different behavior
        // depending on the randomly-seeded HashMap ordering
        for _ in 0..30 {
            let actual_capitalization = do_test_bank_update_rewards_determinism();
            assert_eq!(actual_capitalization, expected_capitalization);
        }
    }

    impl VerifyBankHash {
        fn default_for_test() -> Self {
            Self {
                test_hash_calculation: true,
                can_cached_slot_be_unflushed: false,
                ignore_mismatch: false,
                require_rooted_bank: false,
                run_in_background: false,
                store_hash_raw_data_for_debug: false,
            }
        }
    }

    // Test that purging 0 lamports accounts works.
    #[test]
    fn test_purge_empty_accounts() {
        solana_logger::setup();
        let (genesis_config, mint_keypair) = create_genesis_config(sol_to_lamports(1.));
        let amount = genesis_config.rent.minimum_balance(0);
        let parent = Arc::new(Bank::new_for_tests(&genesis_config));
        let mut bank = parent;
        for _ in 0..10 {
            let blockhash = bank.last_blockhash();
            let pubkey = solana_sdk::pubkey::new_rand();
            let tx = system_transaction::transfer(&mint_keypair, &pubkey, 0, blockhash);
            bank.process_transaction(&tx).unwrap();
            bank.freeze();
            bank.squash();
            bank = Arc::new(new_from_parent(&bank));
        }

        bank.freeze();
        bank.squash();
        bank.force_flush_accounts_cache();
        let hash = bank.update_accounts_hash();
        bank.clean_accounts_for_tests();
        assert_eq!(bank.update_accounts_hash(), hash);

        let bank0 = Arc::new(new_from_parent(&bank));
        let blockhash = bank.last_blockhash();
        let keypair = Keypair::new();
        let tx = system_transaction::transfer(&mint_keypair, &keypair.pubkey(), amount, blockhash);
        bank0.process_transaction(&tx).unwrap();

        let bank1 = Arc::new(new_from_parent(&bank0));
        let pubkey = solana_sdk::pubkey::new_rand();
        let blockhash = bank.last_blockhash();
        let tx = system_transaction::transfer(&keypair, &pubkey, amount, blockhash);
        bank1.process_transaction(&tx).unwrap();

        assert_eq!(
            bank0.get_account(&keypair.pubkey()).unwrap().lamports(),
            amount
        );
        assert_eq!(bank1.get_account(&keypair.pubkey()), None);

        info!("bank0 purge");
        let hash = bank0.update_accounts_hash();
        bank0.clean_accounts_for_tests();
        assert_eq!(bank0.update_accounts_hash(), hash);

        assert_eq!(
            bank0.get_account(&keypair.pubkey()).unwrap().lamports(),
            amount
        );
        assert_eq!(bank1.get_account(&keypair.pubkey()), None);

        info!("bank1 purge");
        bank1.clean_accounts_for_tests();

        assert_eq!(
            bank0.get_account(&keypair.pubkey()).unwrap().lamports(),
            amount
        );
        assert_eq!(bank1.get_account(&keypair.pubkey()), None);

        assert!(bank0.verify_bank_hash(VerifyBankHash::default_for_test()));

        // Squash and then verify hash_internal value
        bank0.freeze();
        bank0.squash();
        assert!(bank0.verify_bank_hash(VerifyBankHash::default_for_test()));

        bank1.freeze();
        bank1.squash();
        bank1.update_accounts_hash();
        assert!(bank1.verify_bank_hash(VerifyBankHash::default_for_test()));

        // keypair should have 0 tokens on both forks
        assert_eq!(bank0.get_account(&keypair.pubkey()), None);
        assert_eq!(bank1.get_account(&keypair.pubkey()), None);
        bank1.force_flush_accounts_cache();
        bank1.clean_accounts_for_tests();

        assert!(bank1.verify_bank_hash(VerifyBankHash::default_for_test()));
    }

    #[test]
    fn test_two_payments_to_one_party() {
        let (genesis_config, mint_keypair) = create_genesis_config(sol_to_lamports(1.));
        let pubkey = solana_sdk::pubkey::new_rand();
        let bank = Bank::new_for_tests(&genesis_config);
        let amount = genesis_config.rent.minimum_balance(0);
        assert_eq!(bank.last_blockhash(), genesis_config.hash());

        bank.transfer(amount, &mint_keypair, &pubkey).unwrap();
        assert_eq!(bank.get_balance(&pubkey), amount);

        bank.transfer(amount * 2, &mint_keypair, &pubkey).unwrap();
        assert_eq!(bank.get_balance(&pubkey), amount * 3);
        assert_eq!(bank.transaction_count(), 2);
    }

    #[test]
    fn test_one_source_two_tx_one_batch() {
        let (genesis_config, mint_keypair) = create_genesis_config(sol_to_lamports(1.));
        let key1 = solana_sdk::pubkey::new_rand();
        let key2 = solana_sdk::pubkey::new_rand();
        let bank = Bank::new_for_tests(&genesis_config);
        let amount = genesis_config.rent.minimum_balance(0);
        assert_eq!(bank.last_blockhash(), genesis_config.hash());

        let t1 = system_transaction::transfer(&mint_keypair, &key1, amount, genesis_config.hash());
        let t2 = system_transaction::transfer(&mint_keypair, &key2, amount, genesis_config.hash());
        let txs = vec![t1.clone(), t2.clone()];
        let res = bank.process_transactions(txs.iter());

        assert_eq!(res.len(), 2);
        assert_eq!(res[0], Ok(()));
        assert_eq!(res[1], Err(TransactionError::AccountInUse));
        assert_eq!(
            bank.get_balance(&mint_keypair.pubkey()),
            sol_to_lamports(1.) - amount
        );
        assert_eq!(bank.get_balance(&key1), amount);
        assert_eq!(bank.get_balance(&key2), 0);
        assert_eq!(bank.get_signature_status(&t1.signatures[0]), Some(Ok(())));
        // TODO: Transactions that fail to pay a fee could be dropped silently.
        // Non-instruction errors don't get logged in the signature cache
        assert_eq!(bank.get_signature_status(&t2.signatures[0]), None);
    }

    #[test]
    fn test_one_tx_two_out_atomic_fail() {
        let amount = sol_to_lamports(1.);
        let (genesis_config, mint_keypair) = create_genesis_config(amount);
        let key1 = solana_sdk::pubkey::new_rand();
        let key2 = solana_sdk::pubkey::new_rand();
        let bank = Bank::new_for_tests(&genesis_config);
        let instructions = system_instruction::transfer_many(
            &mint_keypair.pubkey(),
            &[(key1, amount), (key2, amount)],
        );
        let message = Message::new(&instructions, Some(&mint_keypair.pubkey()));
        let tx = Transaction::new(&[&mint_keypair], message, genesis_config.hash());
        assert_eq!(
            bank.process_transaction(&tx).unwrap_err(),
            TransactionError::InstructionError(1, SystemError::ResultWithNegativeLamports.into())
        );
        assert_eq!(bank.get_balance(&mint_keypair.pubkey()), amount);
        assert_eq!(bank.get_balance(&key1), 0);
        assert_eq!(bank.get_balance(&key2), 0);
    }

    #[test]
    fn test_one_tx_two_out_atomic_pass() {
        let (genesis_config, mint_keypair) = create_genesis_config(sol_to_lamports(1.));
        let key1 = solana_sdk::pubkey::new_rand();
        let key2 = solana_sdk::pubkey::new_rand();
        let bank = Bank::new_for_tests(&genesis_config);
        let amount = genesis_config.rent.minimum_balance(0);
        let instructions = system_instruction::transfer_many(
            &mint_keypair.pubkey(),
            &[(key1, amount), (key2, amount)],
        );
        let message = Message::new(&instructions, Some(&mint_keypair.pubkey()));
        let tx = Transaction::new(&[&mint_keypair], message, genesis_config.hash());
        bank.process_transaction(&tx).unwrap();
        assert_eq!(
            bank.get_balance(&mint_keypair.pubkey()),
            sol_to_lamports(1.) - (2 * amount)
        );
        assert_eq!(bank.get_balance(&key1), amount);
        assert_eq!(bank.get_balance(&key2), amount);
    }

    // This test demonstrates that fees are paid even when a program fails.
    #[test]
    fn test_detect_failed_duplicate_transactions() {
        let (mut genesis_config, mint_keypair) = create_genesis_config(10_000);
        genesis_config.fee_rate_governor = FeeRateGovernor::new(5_000, 0);
        let bank = Bank::new_for_tests(&genesis_config);

        let dest = Keypair::new();

        // source with 0 program context
        let tx = system_transaction::transfer(
            &mint_keypair,
            &dest.pubkey(),
            10_000,
            genesis_config.hash(),
        );
        let signature = tx.signatures[0];
        assert!(!bank.has_signature(&signature));

        assert_eq!(
            bank.process_transaction(&tx),
            Err(TransactionError::InstructionError(
                0,
                SystemError::ResultWithNegativeLamports.into(),
            ))
        );

        // The lamports didn't move, but the from address paid the transaction fee.
        assert_eq!(bank.get_balance(&dest.pubkey()), 0);

        // This should be the original balance minus the transaction fee.
        assert_eq!(bank.get_balance(&mint_keypair.pubkey()), 5000);
    }

    #[test]
    fn test_account_not_found() {
        solana_logger::setup();
        let (genesis_config, mint_keypair) = create_genesis_config(0);
        let bank = Bank::new_for_tests(&genesis_config);
        let keypair = Keypair::new();
        assert_eq!(
            bank.transfer(
                genesis_config.rent.minimum_balance(0),
                &keypair,
                &mint_keypair.pubkey()
            ),
            Err(TransactionError::AccountNotFound)
        );
        assert_eq!(bank.transaction_count(), 0);
    }

    #[test]
    fn test_insufficient_funds() {
        let mint_amount = sol_to_lamports(1.);
        let (genesis_config, mint_keypair) = create_genesis_config(mint_amount);
        let bank = Bank::new_for_tests(&genesis_config);
        let pubkey = solana_sdk::pubkey::new_rand();
        let amount = genesis_config.rent.minimum_balance(0);
        bank.transfer(amount, &mint_keypair, &pubkey).unwrap();
        assert_eq!(bank.transaction_count(), 1);
        assert_eq!(bank.get_balance(&pubkey), amount);
        assert_eq!(
            bank.transfer((mint_amount - amount) + 1, &mint_keypair, &pubkey),
            Err(TransactionError::InstructionError(
                0,
                SystemError::ResultWithNegativeLamports.into(),
            ))
        );
        assert_eq!(bank.transaction_count(), 1);

        let mint_pubkey = mint_keypair.pubkey();
        assert_eq!(bank.get_balance(&mint_pubkey), mint_amount - amount);
        assert_eq!(bank.get_balance(&pubkey), amount);
    }

    #[test]
    fn test_transfer_to_newb() {
        solana_logger::setup();
        let (genesis_config, mint_keypair) = create_genesis_config(sol_to_lamports(1.));
        let bank = Bank::new_for_tests(&genesis_config);
        let amount = genesis_config.rent.minimum_balance(0);
        let pubkey = solana_sdk::pubkey::new_rand();
        bank.transfer(amount, &mint_keypair, &pubkey).unwrap();
        assert_eq!(bank.get_balance(&pubkey), amount);
    }

    #[test]
    fn test_transfer_to_sysvar() {
        solana_logger::setup();
        let (genesis_config, mint_keypair) = create_genesis_config(sol_to_lamports(1.));
        let bank = Arc::new(Bank::new_for_tests(&genesis_config));
        let amount = genesis_config.rent.minimum_balance(0);

        let normal_pubkey = solana_sdk::pubkey::new_rand();
        let sysvar_pubkey = sysvar::clock::id();
        assert_eq!(bank.get_balance(&normal_pubkey), 0);
        assert_eq!(bank.get_balance(&sysvar_pubkey), 1_169_280);

        bank.transfer(amount, &mint_keypair, &normal_pubkey)
            .unwrap();
        bank.transfer(amount, &mint_keypair, &sysvar_pubkey)
            .unwrap_err();
        assert_eq!(bank.get_balance(&normal_pubkey), amount);
        assert_eq!(bank.get_balance(&sysvar_pubkey), 1_169_280);

        let bank = Arc::new(new_from_parent(&bank));
        assert_eq!(bank.get_balance(&normal_pubkey), amount);
        assert_eq!(bank.get_balance(&sysvar_pubkey), 1_169_280);
    }

    #[test]
    fn test_bank_deposit() {
        let bank = create_simple_test_bank(100);

        // Test new account
        let key = solana_sdk::pubkey::new_rand();
        let new_balance = bank.deposit(&key, 10).unwrap();
        assert_eq!(new_balance, 10);
        assert_eq!(bank.get_balance(&key), 10);

        // Existing account
        let new_balance = bank.deposit(&key, 3).unwrap();
        assert_eq!(new_balance, 13);
        assert_eq!(bank.get_balance(&key), 13);
    }

    #[test]
    fn test_bank_withdraw() {
        let bank = create_simple_test_bank(100);

        // Test no account
        let key = solana_sdk::pubkey::new_rand();
        assert_eq!(
            bank.withdraw(&key, 10),
            Err(TransactionError::AccountNotFound)
        );

        bank.deposit(&key, 3).unwrap();
        assert_eq!(bank.get_balance(&key), 3);

        // Low balance
        assert_eq!(
            bank.withdraw(&key, 10),
            Err(TransactionError::InsufficientFundsForFee)
        );

        // Enough balance
        assert_eq!(bank.withdraw(&key, 2), Ok(()));
        assert_eq!(bank.get_balance(&key), 1);
    }

    #[test]
    fn test_bank_withdraw_from_nonce_account() {
        let (mut genesis_config, _mint_keypair) = create_genesis_config(100_000);
        genesis_config.rent.lamports_per_byte_year = 42;
        let bank = Bank::new_for_tests(&genesis_config);

        let min_balance = bank.get_minimum_balance_for_rent_exemption(nonce::State::size());
        let nonce = Keypair::new();
        let nonce_account = AccountSharedData::new_data(
            min_balance + 42,
            &nonce::state::Versions::new(nonce::State::Initialized(nonce::state::Data::default())),
            &system_program::id(),
        )
        .unwrap();
        bank.store_account(&nonce.pubkey(), &nonce_account);
        assert_eq!(bank.get_balance(&nonce.pubkey()), min_balance + 42);

        // Resulting in non-zero, but sub-min_balance balance fails
        assert_eq!(
            bank.withdraw(&nonce.pubkey(), min_balance / 2),
            Err(TransactionError::InsufficientFundsForFee)
        );
        assert_eq!(bank.get_balance(&nonce.pubkey()), min_balance + 42);

        // Resulting in exactly rent-exempt balance succeeds
        bank.withdraw(&nonce.pubkey(), 42).unwrap();
        assert_eq!(bank.get_balance(&nonce.pubkey()), min_balance);

        // Account closure fails
        assert_eq!(
            bank.withdraw(&nonce.pubkey(), min_balance),
            Err(TransactionError::InsufficientFundsForFee),
        );
    }

    #[test]
    fn test_bank_tx_fee() {
        solana_logger::setup();

        let arbitrary_transfer_amount = 42_000;
        let mint = arbitrary_transfer_amount * 100;
        let leader = solana_sdk::pubkey::new_rand();
        let GenesisConfigInfo {
            mut genesis_config,
            mint_keypair,
            ..
        } = create_genesis_config_with_leader(mint, &leader, 3);
        genesis_config.fee_rate_governor = FeeRateGovernor::new(5000, 0); // something divisible by 2

        let expected_fee_paid = genesis_config
            .fee_rate_governor
            .create_fee_calculator()
            .lamports_per_signature;
        let (expected_fee_collected, expected_fee_burned) =
            genesis_config.fee_rate_governor.burn(expected_fee_paid);

        let mut bank = Bank::new_for_tests(&genesis_config);

        let capitalization = bank.capitalization();

        let key = solana_sdk::pubkey::new_rand();
        let tx = system_transaction::transfer(
            &mint_keypair,
            &key,
            arbitrary_transfer_amount,
            bank.last_blockhash(),
        );

        let initial_balance = bank.get_balance(&leader);
        assert_eq!(bank.process_transaction(&tx), Ok(()));
        assert_eq!(bank.get_balance(&key), arbitrary_transfer_amount);
        assert_eq!(
            bank.get_balance(&mint_keypair.pubkey()),
            mint - arbitrary_transfer_amount - expected_fee_paid
        );

        assert_eq!(bank.get_balance(&leader), initial_balance);
        goto_end_of_slot(&mut bank);
        assert_eq!(bank.signature_count(), 1);
        assert_eq!(
            bank.get_balance(&leader),
            initial_balance + expected_fee_collected
        ); // Leader collects fee after the bank is frozen

        // verify capitalization
        let sysvar_and_builtin_program_delta = 1;
        assert_eq!(
            capitalization - expected_fee_burned + sysvar_and_builtin_program_delta,
            bank.capitalization()
        );

        assert_eq!(
            *bank.rewards.read().unwrap(),
            vec![(
                leader,
                RewardInfo {
                    reward_type: RewardType::Fee,
                    lamports: expected_fee_collected as i64,
                    post_balance: initial_balance + expected_fee_collected,
                    commission: None,
                }
            )]
        );

        // Verify that an InstructionError collects fees, too
        let mut bank = Bank::new_from_parent(&Arc::new(bank), &leader, 1);
        let mut tx = system_transaction::transfer(&mint_keypair, &key, 1, bank.last_blockhash());
        // Create a bogus instruction to system_program to cause an instruction error
        tx.message.instructions[0].data[0] = 40;

        bank.process_transaction(&tx)
            .expect_err("instruction error");
        assert_eq!(bank.get_balance(&key), arbitrary_transfer_amount); // no change
        assert_eq!(
            bank.get_balance(&mint_keypair.pubkey()),
            mint - arbitrary_transfer_amount - 2 * expected_fee_paid
        ); // mint_keypair still pays a fee
        goto_end_of_slot(&mut bank);
        assert_eq!(bank.signature_count(), 1);

        // Profit! 2 transaction signatures processed at 3 lamports each
        assert_eq!(
            bank.get_balance(&leader),
            initial_balance + 2 * expected_fee_collected
        );

        assert_eq!(
            *bank.rewards.read().unwrap(),
            vec![(
                leader,
                RewardInfo {
                    reward_type: RewardType::Fee,
                    lamports: expected_fee_collected as i64,
                    post_balance: initial_balance + 2 * expected_fee_collected,
                    commission: None,
                }
            )]
        );
    }

    #[test]
    fn test_bank_tx_compute_unit_fee() {
        solana_logger::setup();

        let key = solana_sdk::pubkey::new_rand();
        let arbitrary_transfer_amount = 42;
        let mint = arbitrary_transfer_amount * 10_000_000;
        let leader = solana_sdk::pubkey::new_rand();
        let GenesisConfigInfo {
            mut genesis_config,
            mint_keypair,
            ..
        } = create_genesis_config_with_leader(mint, &leader, 3);
        genesis_config.fee_rate_governor = FeeRateGovernor::new(4, 0); // something divisible by 2

        let expected_fee_paid = Bank::calculate_fee(
            &SanitizedMessage::try_from(Message::new(&[], Some(&Pubkey::new_unique()))).unwrap(),
            genesis_config
                .fee_rate_governor
                .create_fee_calculator()
                .lamports_per_signature,
            &FeeStructure::default(),
            true,
            false,
        );

        let (expected_fee_collected, expected_fee_burned) =
            genesis_config.fee_rate_governor.burn(expected_fee_paid);

        let mut bank = Bank::new_for_tests(&genesis_config);

        let capitalization = bank.capitalization();

        let tx = system_transaction::transfer(
            &mint_keypair,
            &key,
            arbitrary_transfer_amount,
            bank.last_blockhash(),
        );

        let initial_balance = bank.get_balance(&leader);
        assert_eq!(bank.process_transaction(&tx), Ok(()));
        assert_eq!(bank.get_balance(&key), arbitrary_transfer_amount);
        assert_eq!(
            bank.get_balance(&mint_keypair.pubkey()),
            mint - arbitrary_transfer_amount - expected_fee_paid
        );

        assert_eq!(bank.get_balance(&leader), initial_balance);
        goto_end_of_slot(&mut bank);
        assert_eq!(bank.signature_count(), 1);
        assert_eq!(
            bank.get_balance(&leader),
            initial_balance + expected_fee_collected
        ); // Leader collects fee after the bank is frozen

        // verify capitalization
        let sysvar_and_builtin_program_delta = 1;
        assert_eq!(
            capitalization - expected_fee_burned + sysvar_and_builtin_program_delta,
            bank.capitalization()
        );

        assert_eq!(
            *bank.rewards.read().unwrap(),
            vec![(
                leader,
                RewardInfo {
                    reward_type: RewardType::Fee,
                    lamports: expected_fee_collected as i64,
                    post_balance: initial_balance + expected_fee_collected,
                    commission: None,
                }
            )]
        );

        // Verify that an InstructionError collects fees, too
        let mut bank = Bank::new_from_parent(&Arc::new(bank), &leader, 1);
        let mut tx = system_transaction::transfer(&mint_keypair, &key, 1, bank.last_blockhash());
        // Create a bogus instruction to system_program to cause an instruction error
        tx.message.instructions[0].data[0] = 40;

        bank.process_transaction(&tx)
            .expect_err("instruction error");
        assert_eq!(bank.get_balance(&key), arbitrary_transfer_amount); // no change
        assert_eq!(
            bank.get_balance(&mint_keypair.pubkey()),
            mint - arbitrary_transfer_amount - 2 * expected_fee_paid
        ); // mint_keypair still pays a fee
        goto_end_of_slot(&mut bank);
        assert_eq!(bank.signature_count(), 1);

        // Profit! 2 transaction signatures processed at 3 lamports each
        assert_eq!(
            bank.get_balance(&leader),
            initial_balance + 2 * expected_fee_collected
        );

        assert_eq!(
            *bank.rewards.read().unwrap(),
            vec![(
                leader,
                RewardInfo {
                    reward_type: RewardType::Fee,
                    lamports: expected_fee_collected as i64,
                    post_balance: initial_balance + 2 * expected_fee_collected,
                    commission: None,
                }
            )]
        );
    }

    #[test]
    fn test_bank_blockhash_fee_structure() {
        //solana_logger::setup();

        let leader = solana_sdk::pubkey::new_rand();
        let GenesisConfigInfo {
            mut genesis_config,
            mint_keypair,
            ..
        } = create_genesis_config_with_leader(1_000_000, &leader, 3);
        genesis_config
            .fee_rate_governor
            .target_lamports_per_signature = 5000;
        genesis_config.fee_rate_governor.target_signatures_per_slot = 0;

        let mut bank = Bank::new_for_tests(&genesis_config);
        goto_end_of_slot(&mut bank);
        let cheap_blockhash = bank.last_blockhash();
        let cheap_lamports_per_signature = bank.get_lamports_per_signature();
        assert_eq!(cheap_lamports_per_signature, 0);

        let mut bank = Bank::new_from_parent(&Arc::new(bank), &leader, 1);
        goto_end_of_slot(&mut bank);
        let expensive_blockhash = bank.last_blockhash();
        let expensive_lamports_per_signature = bank.get_lamports_per_signature();
        assert!(cheap_lamports_per_signature < expensive_lamports_per_signature);

        let bank = Bank::new_from_parent(&Arc::new(bank), &leader, 2);

        // Send a transfer using cheap_blockhash
        let key = solana_sdk::pubkey::new_rand();
        let initial_mint_balance = bank.get_balance(&mint_keypair.pubkey());
        let tx = system_transaction::transfer(&mint_keypair, &key, 1, cheap_blockhash);
        assert_eq!(bank.process_transaction(&tx), Ok(()));
        assert_eq!(bank.get_balance(&key), 1);
        assert_eq!(
            bank.get_balance(&mint_keypair.pubkey()),
            initial_mint_balance - 1 - cheap_lamports_per_signature
        );

        // Send a transfer using expensive_blockhash
        let key = solana_sdk::pubkey::new_rand();
        let initial_mint_balance = bank.get_balance(&mint_keypair.pubkey());
        let tx = system_transaction::transfer(&mint_keypair, &key, 1, expensive_blockhash);
        assert_eq!(bank.process_transaction(&tx), Ok(()));
        assert_eq!(bank.get_balance(&key), 1);
        assert_eq!(
            bank.get_balance(&mint_keypair.pubkey()),
            initial_mint_balance - 1 - expensive_lamports_per_signature
        );
    }

    #[test]
    fn test_bank_blockhash_compute_unit_fee_structure() {
        //solana_logger::setup();

        let leader = solana_sdk::pubkey::new_rand();
        let GenesisConfigInfo {
            mut genesis_config,
            mint_keypair,
            ..
        } = create_genesis_config_with_leader(1_000_000_000, &leader, 3);
        genesis_config
            .fee_rate_governor
            .target_lamports_per_signature = 1000;
        genesis_config.fee_rate_governor.target_signatures_per_slot = 1;

        let mut bank = Bank::new_for_tests(&genesis_config);
        goto_end_of_slot(&mut bank);
        let cheap_blockhash = bank.last_blockhash();
        let cheap_lamports_per_signature = bank.get_lamports_per_signature();
        assert_eq!(cheap_lamports_per_signature, 0);

        let mut bank = Bank::new_from_parent(&Arc::new(bank), &leader, 1);
        goto_end_of_slot(&mut bank);
        let expensive_blockhash = bank.last_blockhash();
        let expensive_lamports_per_signature = bank.get_lamports_per_signature();
        assert!(cheap_lamports_per_signature < expensive_lamports_per_signature);

        let bank = Bank::new_from_parent(&Arc::new(bank), &leader, 2);

        // Send a transfer using cheap_blockhash
        let key = solana_sdk::pubkey::new_rand();
        let initial_mint_balance = bank.get_balance(&mint_keypair.pubkey());
        let tx = system_transaction::transfer(&mint_keypair, &key, 1, cheap_blockhash);
        assert_eq!(bank.process_transaction(&tx), Ok(()));
        assert_eq!(bank.get_balance(&key), 1);
        let cheap_fee = Bank::calculate_fee(
            &SanitizedMessage::try_from(Message::new(&[], Some(&Pubkey::new_unique()))).unwrap(),
            cheap_lamports_per_signature,
            &FeeStructure::default(),
            true,
            false,
        );
        assert_eq!(
            bank.get_balance(&mint_keypair.pubkey()),
            initial_mint_balance - 1 - cheap_fee
        );

        // Send a transfer using expensive_blockhash
        let key = solana_sdk::pubkey::new_rand();
        let initial_mint_balance = bank.get_balance(&mint_keypair.pubkey());
        let tx = system_transaction::transfer(&mint_keypair, &key, 1, expensive_blockhash);
        assert_eq!(bank.process_transaction(&tx), Ok(()));
        assert_eq!(bank.get_balance(&key), 1);
        let expensive_fee = Bank::calculate_fee(
            &SanitizedMessage::try_from(Message::new(&[], Some(&Pubkey::new_unique()))).unwrap(),
            expensive_lamports_per_signature,
            &FeeStructure::default(),
            true,
            false,
        );
        assert_eq!(
            bank.get_balance(&mint_keypair.pubkey()),
            initial_mint_balance - 1 - expensive_fee
        );
    }

    #[test]
    fn test_filter_program_errors_and_collect_fee() {
        let leader = solana_sdk::pubkey::new_rand();
        let GenesisConfigInfo {
            mut genesis_config,
            mint_keypair,
            ..
        } = create_genesis_config_with_leader(100_000, &leader, 3);
        genesis_config.fee_rate_governor = FeeRateGovernor::new(5000, 0);
        let bank = Bank::new_for_tests(&genesis_config);

        let key = solana_sdk::pubkey::new_rand();
        let tx1 = SanitizedTransaction::from_transaction_for_tests(system_transaction::transfer(
            &mint_keypair,
            &key,
            2,
            genesis_config.hash(),
        ));
        let tx2 = SanitizedTransaction::from_transaction_for_tests(system_transaction::transfer(
            &mint_keypair,
            &key,
            5,
            genesis_config.hash(),
        ));

        let results = vec![
            new_execution_result(Ok(()), None),
            new_execution_result(
                Err(TransactionError::InstructionError(
                    1,
                    SystemError::ResultWithNegativeLamports.into(),
                )),
                None,
            ),
        ];
        let initial_balance = bank.get_balance(&leader);

        let results = bank.filter_program_errors_and_collect_fee(&[tx1, tx2], &results);
        bank.freeze();
        assert_eq!(
            bank.get_balance(&leader),
            initial_balance
                + bank
                    .fee_rate_governor
                    .burn(bank.fee_rate_governor.lamports_per_signature * 2)
                    .0
        );
        assert_eq!(results[0], Ok(()));
        assert_eq!(results[1], Ok(()));
    }

    #[test]
    fn test_filter_program_errors_and_collect_compute_unit_fee() {
        let leader = solana_sdk::pubkey::new_rand();
        let GenesisConfigInfo {
            mut genesis_config,
            mint_keypair,
            ..
        } = create_genesis_config_with_leader(1000000, &leader, 3);
        genesis_config.fee_rate_governor = FeeRateGovernor::new(2, 0);
        let bank = Bank::new_for_tests(&genesis_config);

        let key = solana_sdk::pubkey::new_rand();
        let tx1 = SanitizedTransaction::from_transaction_for_tests(system_transaction::transfer(
            &mint_keypair,
            &key,
            2,
            genesis_config.hash(),
        ));
        let tx2 = SanitizedTransaction::from_transaction_for_tests(system_transaction::transfer(
            &mint_keypair,
            &key,
            5,
            genesis_config.hash(),
        ));

        let results = vec![
            new_execution_result(Ok(()), None),
            new_execution_result(
                Err(TransactionError::InstructionError(
                    1,
                    SystemError::ResultWithNegativeLamports.into(),
                )),
                None,
            ),
        ];
        let initial_balance = bank.get_balance(&leader);

        let results = bank.filter_program_errors_and_collect_fee(&[tx1, tx2], &results);
        bank.freeze();
        assert_eq!(
            bank.get_balance(&leader),
            initial_balance
                + bank
                    .fee_rate_governor
                    .burn(
                        Bank::calculate_fee(
                            &SanitizedMessage::try_from(Message::new(
                                &[],
                                Some(&Pubkey::new_unique())
                            ))
                            .unwrap(),
                            genesis_config
                                .fee_rate_governor
                                .create_fee_calculator()
                                .lamports_per_signature,
                            &FeeStructure::default(),
                            true,
                            false,
                        ) * 2
                    )
                    .0
        );
        assert_eq!(results[0], Ok(()));
        assert_eq!(results[1], Ok(()));
    }

    #[test]
    fn test_debits_before_credits() {
        let (genesis_config, mint_keypair) = create_genesis_config(sol_to_lamports(2.));
        let bank = Bank::new_for_tests(&genesis_config);
        let keypair = Keypair::new();
        let tx0 = system_transaction::transfer(
            &mint_keypair,
            &keypair.pubkey(),
            sol_to_lamports(2.),
            genesis_config.hash(),
        );
        let tx1 = system_transaction::transfer(
            &keypair,
            &mint_keypair.pubkey(),
            sol_to_lamports(1.),
            genesis_config.hash(),
        );
        let txs = vec![tx0, tx1];
        let results = bank.process_transactions(txs.iter());
        assert!(results[1].is_err());

        // Assert bad transactions aren't counted.
        assert_eq!(bank.transaction_count(), 1);
    }

    #[test]
    fn test_readonly_accounts() {
        let GenesisConfigInfo {
            genesis_config,
            mint_keypair,
            ..
        } = create_genesis_config_with_leader(500, &solana_sdk::pubkey::new_rand(), 0);
        let bank = Bank::new_for_tests(&genesis_config);

        let vote_pubkey0 = solana_sdk::pubkey::new_rand();
        let vote_pubkey1 = solana_sdk::pubkey::new_rand();
        let vote_pubkey2 = solana_sdk::pubkey::new_rand();
        let authorized_voter = Keypair::new();
        let payer0 = Keypair::new();
        let payer1 = Keypair::new();

        // Create vote accounts
        let vote_account0 =
            vote_state::create_account(&vote_pubkey0, &authorized_voter.pubkey(), 0, 100);
        let vote_account1 =
            vote_state::create_account(&vote_pubkey1, &authorized_voter.pubkey(), 0, 100);
        let vote_account2 =
            vote_state::create_account(&vote_pubkey2, &authorized_voter.pubkey(), 0, 100);
        bank.store_account(&vote_pubkey0, &vote_account0);
        bank.store_account(&vote_pubkey1, &vote_account1);
        bank.store_account(&vote_pubkey2, &vote_account2);

        // Fund payers
        bank.transfer(10, &mint_keypair, &payer0.pubkey()).unwrap();
        bank.transfer(10, &mint_keypair, &payer1.pubkey()).unwrap();
        bank.transfer(1, &mint_keypair, &authorized_voter.pubkey())
            .unwrap();

        let vote = Vote::new(vec![1], Hash::default());
        let ix0 = vote_instruction::vote(&vote_pubkey0, &authorized_voter.pubkey(), vote.clone());
        let tx0 = Transaction::new_signed_with_payer(
            &[ix0],
            Some(&payer0.pubkey()),
            &[&payer0, &authorized_voter],
            bank.last_blockhash(),
        );
        let ix1 = vote_instruction::vote(&vote_pubkey1, &authorized_voter.pubkey(), vote.clone());
        let tx1 = Transaction::new_signed_with_payer(
            &[ix1],
            Some(&payer1.pubkey()),
            &[&payer1, &authorized_voter],
            bank.last_blockhash(),
        );
        let txs = vec![tx0, tx1];
        let results = bank.process_transactions(txs.iter());

        // If multiple transactions attempt to read the same account, they should succeed.
        // Vote authorized_voter and sysvar accounts are given read-only handling
        assert_eq!(results[0], Ok(()));
        assert_eq!(results[1], Ok(()));

        let ix0 = vote_instruction::vote(&vote_pubkey2, &authorized_voter.pubkey(), vote);
        let tx0 = Transaction::new_signed_with_payer(
            &[ix0],
            Some(&payer0.pubkey()),
            &[&payer0, &authorized_voter],
            bank.last_blockhash(),
        );
        let tx1 = system_transaction::transfer(
            &authorized_voter,
            &solana_sdk::pubkey::new_rand(),
            1,
            bank.last_blockhash(),
        );
        let txs = vec![tx0, tx1];
        let results = bank.process_transactions(txs.iter());
        // However, an account may not be locked as read-only and writable at the same time.
        assert_eq!(results[0], Ok(()));
        assert_eq!(results[1], Err(TransactionError::AccountInUse));
    }

    #[test]
    fn test_interleaving_locks() {
        let (genesis_config, mint_keypair) = create_genesis_config(sol_to_lamports(1.));
        let bank = Bank::new_for_tests(&genesis_config);
        let alice = Keypair::new();
        let bob = Keypair::new();
        let amount = genesis_config.rent.minimum_balance(0);

        let tx1 = system_transaction::transfer(
            &mint_keypair,
            &alice.pubkey(),
            amount,
            genesis_config.hash(),
        );
        let pay_alice = vec![tx1];

        let lock_result = bank.prepare_batch_for_tests(pay_alice);
        let results_alice = bank
            .load_execute_and_commit_transactions(
                &lock_result,
                MAX_PROCESSING_AGE,
                false,
                false,
                false,
                false,
                &mut ExecuteTimings::default(),
                None,
            )
            .0
            .fee_collection_results;
        assert_eq!(results_alice[0], Ok(()));

        // try executing an interleaved transfer twice
        assert_eq!(
            bank.transfer(amount, &mint_keypair, &bob.pubkey()),
            Err(TransactionError::AccountInUse)
        );
        // the second time should fail as well
        // this verifies that `unlock_accounts` doesn't unlock `AccountInUse` accounts
        assert_eq!(
            bank.transfer(amount, &mint_keypair, &bob.pubkey()),
            Err(TransactionError::AccountInUse)
        );

        drop(lock_result);

        assert!(bank
            .transfer(2 * amount, &mint_keypair, &bob.pubkey())
            .is_ok());
    }

    #[test]
    fn test_readonly_relaxed_locks() {
        let (genesis_config, _) = create_genesis_config(3);
        let bank = Bank::new_for_tests(&genesis_config);
        let key0 = Keypair::new();
        let key1 = Keypair::new();
        let key2 = Keypair::new();
        let key3 = solana_sdk::pubkey::new_rand();

        let message = Message {
            header: MessageHeader {
                num_required_signatures: 1,
                num_readonly_signed_accounts: 0,
                num_readonly_unsigned_accounts: 1,
            },
            account_keys: vec![key0.pubkey(), key3],
            recent_blockhash: Hash::default(),
            instructions: vec![],
        };
        let tx = Transaction::new(&[&key0], message, genesis_config.hash());
        let txs = vec![tx];

        let batch0 = bank.prepare_batch_for_tests(txs);
        assert!(batch0.lock_results()[0].is_ok());

        // Try locking accounts, locking a previously read-only account as writable
        // should fail
        let message = Message {
            header: MessageHeader {
                num_required_signatures: 1,
                num_readonly_signed_accounts: 0,
                num_readonly_unsigned_accounts: 0,
            },
            account_keys: vec![key1.pubkey(), key3],
            recent_blockhash: Hash::default(),
            instructions: vec![],
        };
        let tx = Transaction::new(&[&key1], message, genesis_config.hash());
        let txs = vec![tx];

        let batch1 = bank.prepare_batch_for_tests(txs);
        assert!(batch1.lock_results()[0].is_err());

        // Try locking a previously read-only account a 2nd time; should succeed
        let message = Message {
            header: MessageHeader {
                num_required_signatures: 1,
                num_readonly_signed_accounts: 0,
                num_readonly_unsigned_accounts: 1,
            },
            account_keys: vec![key2.pubkey(), key3],
            recent_blockhash: Hash::default(),
            instructions: vec![],
        };
        let tx = Transaction::new(&[&key2], message, genesis_config.hash());
        let txs = vec![tx];

        let batch2 = bank.prepare_batch_for_tests(txs);
        assert!(batch2.lock_results()[0].is_ok());
    }

    #[test]
    fn test_bank_invalid_account_index() {
        let (genesis_config, mint_keypair) = create_genesis_config(1);
        let keypair = Keypair::new();
        let bank = Bank::new_for_tests(&genesis_config);

        let tx = system_transaction::transfer(
            &mint_keypair,
            &keypair.pubkey(),
            1,
            genesis_config.hash(),
        );

        let mut tx_invalid_program_index = tx.clone();
        tx_invalid_program_index.message.instructions[0].program_id_index = 42;
        assert_eq!(
            bank.process_transaction(&tx_invalid_program_index),
            Err(TransactionError::SanitizeFailure)
        );

        let mut tx_invalid_account_index = tx;
        tx_invalid_account_index.message.instructions[0].accounts[0] = 42;
        assert_eq!(
            bank.process_transaction(&tx_invalid_account_index),
            Err(TransactionError::SanitizeFailure)
        );
    }

    #[test]
    fn test_bank_pay_to_self() {
        let (genesis_config, mint_keypair) = create_genesis_config(sol_to_lamports(1.));
        let key1 = Keypair::new();
        let bank = Bank::new_for_tests(&genesis_config);
        let amount = genesis_config.rent.minimum_balance(0);

        bank.transfer(amount, &mint_keypair, &key1.pubkey())
            .unwrap();
        assert_eq!(bank.get_balance(&key1.pubkey()), amount);
        let tx = system_transaction::transfer(&key1, &key1.pubkey(), amount, genesis_config.hash());
        let _res = bank.process_transaction(&tx);

        assert_eq!(bank.get_balance(&key1.pubkey()), amount);
        bank.get_signature_status(&tx.signatures[0])
            .unwrap()
            .unwrap();
    }

    fn new_from_parent(parent: &Arc<Bank>) -> Bank {
        Bank::new_from_parent(parent, &Pubkey::default(), parent.slot() + 1)
    }

    /// Verify that the parent's vector is computed correctly
    #[test]
    fn test_bank_parents() {
        let (genesis_config, _) = create_genesis_config(1);
        let parent = Arc::new(Bank::new_for_tests(&genesis_config));

        let bank = new_from_parent(&parent);
        assert!(Arc::ptr_eq(&bank.parents()[0], &parent));
    }

    /// Verifies that transactions are dropped if they have already been processed
    #[test]
    fn test_tx_already_processed() {
        let (genesis_config, mint_keypair) = create_genesis_config(sol_to_lamports(1.));
        let bank = Bank::new_for_tests(&genesis_config);

        let key1 = Keypair::new();
        let mut tx = system_transaction::transfer(
            &mint_keypair,
            &key1.pubkey(),
            genesis_config.rent.minimum_balance(0),
            genesis_config.hash(),
        );

        // First process `tx` so that the status cache is updated
        assert_eq!(bank.process_transaction(&tx), Ok(()));

        // Ensure that signature check works
        assert_eq!(
            bank.process_transaction(&tx),
            Err(TransactionError::AlreadyProcessed)
        );

        // Change transaction signature to simulate processing a transaction with a different signature
        // for the same message.
        tx.signatures[0] = Signature::default();

        // Ensure that message hash check works
        assert_eq!(
            bank.process_transaction(&tx),
            Err(TransactionError::AlreadyProcessed)
        );
    }

    /// Verifies that last ids and status cache are correctly referenced from parent
    #[test]
    fn test_bank_parent_already_processed() {
        let (genesis_config, mint_keypair) = create_genesis_config(sol_to_lamports(1.));
        let key1 = Keypair::new();
        let parent = Arc::new(Bank::new_for_tests(&genesis_config));
        let amount = genesis_config.rent.minimum_balance(0);

        let tx = system_transaction::transfer(
            &mint_keypair,
            &key1.pubkey(),
            amount,
            genesis_config.hash(),
        );
        assert_eq!(parent.process_transaction(&tx), Ok(()));
        let bank = new_from_parent(&parent);
        assert_eq!(
            bank.process_transaction(&tx),
            Err(TransactionError::AlreadyProcessed)
        );
    }

    /// Verifies that last ids and accounts are correctly referenced from parent
    #[test]
    fn test_bank_parent_account_spend() {
        let (genesis_config, mint_keypair) = create_genesis_config(sol_to_lamports(1.0));
        let key1 = Keypair::new();
        let key2 = Keypair::new();
        let parent = Arc::new(Bank::new_for_tests(&genesis_config));
        let amount = genesis_config.rent.minimum_balance(0);

        let tx = system_transaction::transfer(
            &mint_keypair,
            &key1.pubkey(),
            amount,
            genesis_config.hash(),
        );
        assert_eq!(parent.process_transaction(&tx), Ok(()));
        let bank = new_from_parent(&parent);
        let tx = system_transaction::transfer(&key1, &key2.pubkey(), amount, genesis_config.hash());
        assert_eq!(bank.process_transaction(&tx), Ok(()));
        assert_eq!(parent.get_signature_status(&tx.signatures[0]), None);
    }

    #[test]
    fn test_bank_hash_internal_state() {
        let (genesis_config, mint_keypair) = create_genesis_config(sol_to_lamports(1.));
        let bank0 = Bank::new_for_tests(&genesis_config);
        let bank1 = Bank::new_for_tests(&genesis_config);
        let amount = genesis_config.rent.minimum_balance(0);
        let initial_state = bank0.hash_internal_state();
        assert_eq!(bank1.hash_internal_state(), initial_state);

        let pubkey = solana_sdk::pubkey::new_rand();
        bank0.transfer(amount, &mint_keypair, &pubkey).unwrap();
        assert_ne!(bank0.hash_internal_state(), initial_state);
        bank1.transfer(amount, &mint_keypair, &pubkey).unwrap();
        assert_eq!(bank0.hash_internal_state(), bank1.hash_internal_state());

        // Checkpointing should always result in a new state
        let bank2 = new_from_parent(&Arc::new(bank1));
        assert_ne!(bank0.hash_internal_state(), bank2.hash_internal_state());

        let pubkey2 = solana_sdk::pubkey::new_rand();
        info!("transfer 2 {}", pubkey2);
        bank2.transfer(amount, &mint_keypair, &pubkey2).unwrap();
        bank2.update_accounts_hash();
        assert!(bank2.verify_bank_hash(VerifyBankHash::default_for_test()));
    }

    #[test]
    fn test_bank_hash_internal_state_verify() {
        solana_logger::setup();
        let (genesis_config, mint_keypair) = create_genesis_config(sol_to_lamports(1.));
        let bank0 = Bank::new_for_tests(&genesis_config);
        let amount = genesis_config.rent.minimum_balance(0);

        let pubkey = solana_sdk::pubkey::new_rand();
        info!("transfer 0 {} mint: {}", pubkey, mint_keypair.pubkey());
        bank0.transfer(amount, &mint_keypair, &pubkey).unwrap();

        let bank0_state = bank0.hash_internal_state();
        let bank0 = Arc::new(bank0);
        // Checkpointing should result in a new state while freezing the parent
        let bank2 = Bank::new_from_parent(&bank0, &solana_sdk::pubkey::new_rand(), 1);
        assert_ne!(bank0_state, bank2.hash_internal_state());
        // Checkpointing should modify the checkpoint's state when freezed
        assert_ne!(bank0_state, bank0.hash_internal_state());

        // Checkpointing should never modify the checkpoint's state once frozen
        let bank0_state = bank0.hash_internal_state();
        bank2.update_accounts_hash();
        assert!(bank2.verify_bank_hash(VerifyBankHash::default_for_test()));
        let bank3 = Bank::new_from_parent(&bank0, &solana_sdk::pubkey::new_rand(), 2);
        assert_eq!(bank0_state, bank0.hash_internal_state());
        assert!(bank2.verify_bank_hash(VerifyBankHash::default_for_test()));
        bank3.update_accounts_hash();
        assert!(bank3.verify_bank_hash(VerifyBankHash::default_for_test()));

        let pubkey2 = solana_sdk::pubkey::new_rand();
        info!("transfer 2 {}", pubkey2);
        bank2.transfer(amount, &mint_keypair, &pubkey2).unwrap();
        bank2.update_accounts_hash();
        assert!(bank2.verify_bank_hash(VerifyBankHash::default_for_test()));
        assert!(bank3.verify_bank_hash(VerifyBankHash::default_for_test()));
    }

    #[test]
    #[should_panic(expected = "assertion failed: self.is_frozen()")]
    fn test_verify_hash_unfrozen() {
        let bank = create_simple_test_bank(2_000);
        assert!(bank.verify_hash());
    }

    #[test]
    fn test_verify_snapshot_bank() {
        solana_logger::setup();
        let pubkey = solana_sdk::pubkey::new_rand();
        let (genesis_config, mint_keypair) = create_genesis_config(sol_to_lamports(1.));
        let bank = Bank::new_for_tests(&genesis_config);
        bank.transfer(
            genesis_config.rent.minimum_balance(0),
            &mint_keypair,
            &pubkey,
        )
        .unwrap();
        bank.freeze();
        bank.update_accounts_hash();
        assert!(bank.verify_snapshot_bank(true, false, bank.slot()));

        // tamper the bank after freeze!
        bank.increment_signature_count(1);
        assert!(!bank.verify_snapshot_bank(true, false, bank.slot()));
    }

    // Test that two bank forks with the same accounts should not hash to the same value.
    #[test]
    fn test_bank_hash_internal_state_same_account_different_fork() {
        solana_logger::setup();
        let (genesis_config, mint_keypair) = create_genesis_config(sol_to_lamports(1.));
        let amount = genesis_config.rent.minimum_balance(0);
        let bank0 = Arc::new(Bank::new_for_tests(&genesis_config));
        let initial_state = bank0.hash_internal_state();
        let bank1 = Bank::new_from_parent(&bank0, &Pubkey::default(), 1);
        assert_ne!(bank1.hash_internal_state(), initial_state);

        info!("transfer bank1");
        let pubkey = solana_sdk::pubkey::new_rand();
        bank1.transfer(amount, &mint_keypair, &pubkey).unwrap();
        assert_ne!(bank1.hash_internal_state(), initial_state);

        info!("transfer bank2");
        // bank2 should not hash the same as bank1
        let bank2 = Bank::new_from_parent(&bank0, &Pubkey::default(), 2);
        bank2.transfer(amount, &mint_keypair, &pubkey).unwrap();
        assert_ne!(bank2.hash_internal_state(), initial_state);
        assert_ne!(bank1.hash_internal_state(), bank2.hash_internal_state());
    }

    #[test]
    fn test_hash_internal_state_genesis() {
        let bank0 = Bank::new_for_tests(&create_genesis_config(10).0);
        let bank1 = Bank::new_for_tests(&create_genesis_config(20).0);
        assert_ne!(bank0.hash_internal_state(), bank1.hash_internal_state());
    }

    // See that the order of two transfers does not affect the result
    // of hash_internal_state
    #[test]
    fn test_hash_internal_state_order() {
        let (genesis_config, mint_keypair) = create_genesis_config(sol_to_lamports(1.));
        let amount = genesis_config.rent.minimum_balance(0);
        let bank0 = Bank::new_for_tests(&genesis_config);
        let bank1 = Bank::new_for_tests(&genesis_config);
        assert_eq!(bank0.hash_internal_state(), bank1.hash_internal_state());
        let key0 = solana_sdk::pubkey::new_rand();
        let key1 = solana_sdk::pubkey::new_rand();
        bank0.transfer(amount, &mint_keypair, &key0).unwrap();
        bank0.transfer(amount * 2, &mint_keypair, &key1).unwrap();

        bank1.transfer(amount * 2, &mint_keypair, &key1).unwrap();
        bank1.transfer(amount, &mint_keypair, &key0).unwrap();

        assert_eq!(bank0.hash_internal_state(), bank1.hash_internal_state());
    }

    #[test]
    fn test_hash_internal_state_error() {
        solana_logger::setup();
        let (genesis_config, mint_keypair) = create_genesis_config(sol_to_lamports(1.));
        let amount = genesis_config.rent.minimum_balance(0);
        let bank = Bank::new_for_tests(&genesis_config);
        let key0 = solana_sdk::pubkey::new_rand();
        bank.transfer(amount, &mint_keypair, &key0).unwrap();
        let orig = bank.hash_internal_state();

        // Transfer will error but still take a fee
        assert!(bank
            .transfer(sol_to_lamports(1.), &mint_keypair, &key0)
            .is_err());
        assert_ne!(orig, bank.hash_internal_state());

        let orig = bank.hash_internal_state();
        let empty_keypair = Keypair::new();
        assert!(bank.transfer(amount, &empty_keypair, &key0).is_err());
        assert_eq!(orig, bank.hash_internal_state());
    }

    #[test]
    fn test_bank_hash_internal_state_squash() {
        let collector_id = Pubkey::default();
        let bank0 = Arc::new(Bank::new_for_tests(&create_genesis_config(10).0));
        let hash0 = bank0.hash_internal_state();
        // save hash0 because new_from_parent
        // updates sysvar entries

        let bank1 = Bank::new_from_parent(&bank0, &collector_id, 1);

        // no delta in bank1, hashes should always update
        assert_ne!(hash0, bank1.hash_internal_state());

        // remove parent
        bank1.squash();
        assert!(bank1.parents().is_empty());
    }

    /// Verifies that last ids and accounts are correctly referenced from parent
    #[test]
    fn test_bank_squash() {
        solana_logger::setup();
        let (genesis_config, mint_keypair) = create_genesis_config(sol_to_lamports(2.));
        let key1 = Keypair::new();
        let key2 = Keypair::new();
        let parent = Arc::new(Bank::new_for_tests(&genesis_config));
        let amount = genesis_config.rent.minimum_balance(0);

        let tx_transfer_mint_to_1 = system_transaction::transfer(
            &mint_keypair,
            &key1.pubkey(),
            amount,
            genesis_config.hash(),
        );
        trace!("parent process tx ");
        assert_eq!(parent.process_transaction(&tx_transfer_mint_to_1), Ok(()));
        trace!("done parent process tx ");
        assert_eq!(parent.transaction_count(), 1);
        assert_eq!(
            parent.get_signature_status(&tx_transfer_mint_to_1.signatures[0]),
            Some(Ok(()))
        );

        trace!("new from parent");
        let bank = new_from_parent(&parent);
        trace!("done new from parent");
        assert_eq!(
            bank.get_signature_status(&tx_transfer_mint_to_1.signatures[0]),
            Some(Ok(()))
        );

        assert_eq!(bank.transaction_count(), parent.transaction_count());
        let tx_transfer_1_to_2 =
            system_transaction::transfer(&key1, &key2.pubkey(), amount, genesis_config.hash());
        assert_eq!(bank.process_transaction(&tx_transfer_1_to_2), Ok(()));
        assert_eq!(bank.transaction_count(), 2);
        assert_eq!(parent.transaction_count(), 1);
        assert_eq!(
            parent.get_signature_status(&tx_transfer_1_to_2.signatures[0]),
            None
        );

        for _ in 0..3 {
            // first time these should match what happened above, assert that parents are ok
            assert_eq!(bank.get_balance(&key1.pubkey()), 0);
            assert_eq!(bank.get_account(&key1.pubkey()), None);
            assert_eq!(bank.get_balance(&key2.pubkey()), amount);
            trace!("start");
            assert_eq!(
                bank.get_signature_status(&tx_transfer_mint_to_1.signatures[0]),
                Some(Ok(()))
            );
            assert_eq!(
                bank.get_signature_status(&tx_transfer_1_to_2.signatures[0]),
                Some(Ok(()))
            );

            // works iteration 0, no-ops on iteration 1 and 2
            trace!("SQUASH");
            bank.squash();

            assert_eq!(parent.transaction_count(), 1);
            assert_eq!(bank.transaction_count(), 2);
        }
    }

    #[test]
    fn test_bank_get_account_in_parent_after_squash() {
        let (genesis_config, mint_keypair) = create_genesis_config(sol_to_lamports(1.));
        let parent = Arc::new(Bank::new_for_tests(&genesis_config));
        let amount = genesis_config.rent.minimum_balance(0);

        let key1 = Keypair::new();

        parent
            .transfer(amount, &mint_keypair, &key1.pubkey())
            .unwrap();
        assert_eq!(parent.get_balance(&key1.pubkey()), amount);
        let bank = new_from_parent(&parent);
        bank.squash();
        assert_eq!(parent.get_balance(&key1.pubkey()), amount);
    }

    #[test]
    fn test_bank_get_account_in_parent_after_squash2() {
        solana_logger::setup();
        let (genesis_config, mint_keypair) = create_genesis_config(sol_to_lamports(1.));
        let bank0 = Arc::new(Bank::new_for_tests(&genesis_config));
        let amount = genesis_config.rent.minimum_balance(0);

        let key1 = Keypair::new();

        bank0
            .transfer(amount, &mint_keypair, &key1.pubkey())
            .unwrap();
        assert_eq!(bank0.get_balance(&key1.pubkey()), amount);

        let bank1 = Arc::new(Bank::new_from_parent(&bank0, &Pubkey::default(), 1));
        bank1
            .transfer(3 * amount, &mint_keypair, &key1.pubkey())
            .unwrap();
        let bank2 = Arc::new(Bank::new_from_parent(&bank0, &Pubkey::default(), 2));
        bank2
            .transfer(2 * amount, &mint_keypair, &key1.pubkey())
            .unwrap();
        let bank3 = Arc::new(Bank::new_from_parent(&bank1, &Pubkey::default(), 3));
        bank1.squash();

        // This picks up the values from 1 which is the highest root:
        // TODO: if we need to access rooted banks older than this,
        // need to fix the lookup.
        assert_eq!(bank0.get_balance(&key1.pubkey()), 4 * amount);
        assert_eq!(bank3.get_balance(&key1.pubkey()), 4 * amount);
        assert_eq!(bank2.get_balance(&key1.pubkey()), 3 * amount);
        bank3.squash();
        assert_eq!(bank1.get_balance(&key1.pubkey()), 4 * amount);

        let bank4 = Arc::new(Bank::new_from_parent(&bank3, &Pubkey::default(), 4));
        bank4
            .transfer(4 * amount, &mint_keypair, &key1.pubkey())
            .unwrap();
        assert_eq!(bank4.get_balance(&key1.pubkey()), 8 * amount);
        assert_eq!(bank3.get_balance(&key1.pubkey()), 4 * amount);
        bank4.squash();
        let bank5 = Arc::new(Bank::new_from_parent(&bank4, &Pubkey::default(), 5));
        bank5.squash();
        let bank6 = Arc::new(Bank::new_from_parent(&bank5, &Pubkey::default(), 6));
        bank6.squash();

        // This picks up the values from 4 which is the highest root:
        // TODO: if we need to access rooted banks older than this,
        // need to fix the lookup.
        assert_eq!(bank3.get_balance(&key1.pubkey()), 8 * amount);
        assert_eq!(bank2.get_balance(&key1.pubkey()), 8 * amount);

        assert_eq!(bank4.get_balance(&key1.pubkey()), 8 * amount);
    }

    #[test]
    fn test_bank_get_account_modified_since_parent_with_fixed_root() {
        let pubkey = solana_sdk::pubkey::new_rand();

        let (genesis_config, mint_keypair) = create_genesis_config(sol_to_lamports(1.));
        let amount = genesis_config.rent.minimum_balance(0);
        let bank1 = Arc::new(Bank::new_for_tests(&genesis_config));
        bank1.transfer(amount, &mint_keypair, &pubkey).unwrap();
        let result = bank1.get_account_modified_since_parent_with_fixed_root(&pubkey);
        assert!(result.is_some());
        let (account, slot) = result.unwrap();
        assert_eq!(account.lamports(), amount);
        assert_eq!(slot, 0);

        let bank2 = Arc::new(Bank::new_from_parent(&bank1, &Pubkey::default(), 1));
        assert!(bank2
            .get_account_modified_since_parent_with_fixed_root(&pubkey)
            .is_none());
        bank2.transfer(2 * amount, &mint_keypair, &pubkey).unwrap();
        let result = bank1.get_account_modified_since_parent_with_fixed_root(&pubkey);
        assert!(result.is_some());
        let (account, slot) = result.unwrap();
        assert_eq!(account.lamports(), amount);
        assert_eq!(slot, 0);
        let result = bank2.get_account_modified_since_parent_with_fixed_root(&pubkey);
        assert!(result.is_some());
        let (account, slot) = result.unwrap();
        assert_eq!(account.lamports(), 3 * amount);
        assert_eq!(slot, 1);

        bank1.squash();

        let bank3 = Bank::new_from_parent(&bank2, &Pubkey::default(), 3);
        assert_eq!(
            None,
            bank3.get_account_modified_since_parent_with_fixed_root(&pubkey)
        );
    }

    #[test]
    fn test_bank_update_sysvar_account() {
        use sysvar::clock::Clock;

        let dummy_clock_id = solana_sdk::pubkey::new_rand();
        let dummy_rent_epoch = 44;
        let (mut genesis_config, _mint_keypair) = create_genesis_config(500);

        let expected_previous_slot = 3;
        let mut expected_next_slot = expected_previous_slot + 1;

        // First, initialize the clock sysvar
        activate_all_features(&mut genesis_config);
        let bank1 = Arc::new(Bank::new_for_tests(&genesis_config));
        assert_eq!(bank1.calculate_capitalization(true), bank1.capitalization());

        assert_capitalization_diff(
            &bank1,
            || {
                bank1.update_sysvar_account(&dummy_clock_id, |optional_account| {
                    assert!(optional_account.is_none());

                    let mut account = create_account(
                        &Clock {
                            slot: expected_previous_slot,
                            ..Clock::default()
                        },
                        bank1.inherit_specially_retained_account_fields(optional_account),
                    );
                    account.set_rent_epoch(dummy_rent_epoch);
                    account
                });
                let current_account = bank1.get_account(&dummy_clock_id).unwrap();
                assert_eq!(
                    expected_previous_slot,
                    from_account::<Clock, _>(&current_account).unwrap().slot
                );
                assert_eq!(dummy_rent_epoch, current_account.rent_epoch());
            },
            |old, new| {
                assert_eq!(
                    old + min_rent_exempt_balance_for_sysvars(&bank1, &[sysvar::clock::id()]),
                    new
                );
            },
        );

        assert_capitalization_diff(
            &bank1,
            || {
                bank1.update_sysvar_account(&dummy_clock_id, |optional_account| {
                    assert!(optional_account.is_some());

                    create_account(
                        &Clock {
                            slot: expected_previous_slot,
                            ..Clock::default()
                        },
                        bank1.inherit_specially_retained_account_fields(optional_account),
                    )
                })
            },
            |old, new| {
                // creating new sysvar twice in a slot shouldn't increment capitalization twice
                assert_eq!(old, new);
            },
        );

        // Updating should increment the clock's slot
        let bank2 = Arc::new(Bank::new_from_parent(&bank1, &Pubkey::default(), 1));
        assert_capitalization_diff(
            &bank2,
            || {
                bank2.update_sysvar_account(&dummy_clock_id, |optional_account| {
                    let slot = from_account::<Clock, _>(optional_account.as_ref().unwrap())
                        .unwrap()
                        .slot
                        + 1;

                    create_account(
                        &Clock {
                            slot,
                            ..Clock::default()
                        },
                        bank2.inherit_specially_retained_account_fields(optional_account),
                    )
                });
                let current_account = bank2.get_account(&dummy_clock_id).unwrap();
                assert_eq!(
                    expected_next_slot,
                    from_account::<Clock, _>(&current_account).unwrap().slot
                );
                assert_eq!(dummy_rent_epoch, current_account.rent_epoch());
            },
            |old, new| {
                // if existing, capitalization shouldn't change
                assert_eq!(old, new);
            },
        );

        // Updating again should give bank2's sysvar to the closure not bank1's.
        // Thus, increment expected_next_slot accordingly
        expected_next_slot += 1;
        assert_capitalization_diff(
            &bank2,
            || {
                bank2.update_sysvar_account(&dummy_clock_id, |optional_account| {
                    let slot = from_account::<Clock, _>(optional_account.as_ref().unwrap())
                        .unwrap()
                        .slot
                        + 1;

                    create_account(
                        &Clock {
                            slot,
                            ..Clock::default()
                        },
                        bank2.inherit_specially_retained_account_fields(optional_account),
                    )
                });
                let current_account = bank2.get_account(&dummy_clock_id).unwrap();
                assert_eq!(
                    expected_next_slot,
                    from_account::<Clock, _>(&current_account).unwrap().slot
                );
            },
            |old, new| {
                // updating twice in a slot shouldn't increment capitalization twice
                assert_eq!(old, new);
            },
        );
    }

    #[test]
    fn test_bank_epoch_vote_accounts() {
        let leader_pubkey = solana_sdk::pubkey::new_rand();
        let leader_lamports = 3;
        let mut genesis_config =
            create_genesis_config_with_leader(5, &leader_pubkey, leader_lamports).genesis_config;

        // set this up weird, forces future generation, odd mod(), etc.
        //  this says: "vote_accounts for epoch X should be generated at slot index 3 in epoch X-2...
        const SLOTS_PER_EPOCH: u64 = MINIMUM_SLOTS_PER_EPOCH as u64;
        const LEADER_SCHEDULE_SLOT_OFFSET: u64 = SLOTS_PER_EPOCH * 3 - 3;
        // no warmup allows me to do the normal division stuff below
        genesis_config.epoch_schedule =
            EpochSchedule::custom(SLOTS_PER_EPOCH, LEADER_SCHEDULE_SLOT_OFFSET, false);

        let parent = Arc::new(Bank::new_for_tests(&genesis_config));
        let mut leader_vote_stake: Vec<_> = parent
            .epoch_vote_accounts(0)
            .map(|accounts| {
                accounts
                    .iter()
                    .filter_map(|(pubkey, (stake, account))| {
                        if let Ok(vote_state) = account.vote_state().as_ref() {
                            if vote_state.node_pubkey == leader_pubkey {
                                Some((*pubkey, *stake))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap();
        assert_eq!(leader_vote_stake.len(), 1);
        let (leader_vote_account, leader_stake) = leader_vote_stake.pop().unwrap();
        assert!(leader_stake > 0);

        let leader_stake = Stake {
            delegation: Delegation {
                stake: leader_lamports,
                activation_epoch: std::u64::MAX, // bootstrap
                ..Delegation::default()
            },
            ..Stake::default()
        };

        let mut epoch = 1;
        loop {
            if epoch > LEADER_SCHEDULE_SLOT_OFFSET / SLOTS_PER_EPOCH {
                break;
            }
            let vote_accounts = parent.epoch_vote_accounts(epoch);
            assert!(vote_accounts.is_some());

            // epoch_stakes are a snapshot at the leader_schedule_slot_offset boundary
            //   in the prior epoch (0 in this case)
            assert_eq!(
                leader_stake.stake(0, None),
                vote_accounts.unwrap().get(&leader_vote_account).unwrap().0
            );

            epoch += 1;
        }

        // child crosses epoch boundary and is the first slot in the epoch
        let child = Bank::new_from_parent(
            &parent,
            &leader_pubkey,
            SLOTS_PER_EPOCH - (LEADER_SCHEDULE_SLOT_OFFSET % SLOTS_PER_EPOCH),
        );

        assert!(child.epoch_vote_accounts(epoch).is_some());
        assert_eq!(
            leader_stake.stake(child.epoch(), None),
            child
                .epoch_vote_accounts(epoch)
                .unwrap()
                .get(&leader_vote_account)
                .unwrap()
                .0
        );

        // child crosses epoch boundary but isn't the first slot in the epoch, still
        //  makes an epoch stakes snapshot at 1
        let child = Bank::new_from_parent(
            &parent,
            &leader_pubkey,
            SLOTS_PER_EPOCH - (LEADER_SCHEDULE_SLOT_OFFSET % SLOTS_PER_EPOCH) + 1,
        );
        assert!(child.epoch_vote_accounts(epoch).is_some());
        assert_eq!(
            leader_stake.stake(child.epoch(), None),
            child
                .epoch_vote_accounts(epoch)
                .unwrap()
                .get(&leader_vote_account)
                .unwrap()
                .0
        );
    }

    #[test]
    fn test_zero_signatures() {
        solana_logger::setup();
        let (genesis_config, mint_keypair) = create_genesis_config(500);
        let mut bank = Bank::new_for_tests(&genesis_config);
        bank.fee_rate_governor.lamports_per_signature = 2;
        let key = solana_sdk::pubkey::new_rand();

        let mut transfer_instruction =
            system_instruction::transfer(&mint_keypair.pubkey(), &key, 0);
        transfer_instruction.accounts[0].is_signer = false;
        let message = Message::new(&[transfer_instruction], None);
        let tx = Transaction::new(&[&Keypair::new(); 0], message, bank.last_blockhash());

        assert_eq!(
            bank.process_transaction(&tx),
            Err(TransactionError::SanitizeFailure)
        );
        assert_eq!(bank.get_balance(&key), 0);
    }

    #[test]
    fn test_bank_get_slots_in_epoch() {
        let (genesis_config, _) = create_genesis_config(500);

        let bank = Bank::new_for_tests(&genesis_config);

        assert_eq!(bank.get_slots_in_epoch(0), MINIMUM_SLOTS_PER_EPOCH as u64);
        assert_eq!(
            bank.get_slots_in_epoch(2),
            (MINIMUM_SLOTS_PER_EPOCH * 4) as u64
        );
        assert_eq!(
            bank.get_slots_in_epoch(5000),
            genesis_config.epoch_schedule.slots_per_epoch
        );
    }

    #[test]
    fn test_is_delta_true() {
        let (genesis_config, mint_keypair) = create_genesis_config(sol_to_lamports(1.0));
        let bank = Arc::new(Bank::new_for_tests(&genesis_config));
        let key1 = Keypair::new();
        let tx_transfer_mint_to_1 = system_transaction::transfer(
            &mint_keypair,
            &key1.pubkey(),
            genesis_config.rent.minimum_balance(0),
            genesis_config.hash(),
        );
        assert_eq!(bank.process_transaction(&tx_transfer_mint_to_1), Ok(()));
        assert!(bank.is_delta.load(Relaxed));

        let bank1 = new_from_parent(&bank);
        let hash1 = bank1.hash_internal_state();
        assert!(!bank1.is_delta.load(Relaxed));
        assert_ne!(hash1, bank.hash());
        // ticks don't make a bank into a delta or change its state unless a block boundary is crossed
        bank1.register_tick(&Hash::default());
        assert!(!bank1.is_delta.load(Relaxed));
        assert_eq!(bank1.hash_internal_state(), hash1);
    }

    #[test]
    fn test_is_empty() {
        let (genesis_config, mint_keypair) = create_genesis_config(sol_to_lamports(1.0));
        let bank0 = Arc::new(Bank::new_for_tests(&genesis_config));
        let key1 = Keypair::new();

        // The zeroth bank is empty becasue there are no transactions
        assert!(bank0.is_empty());

        // Set is_delta to true, bank is no longer empty
        let tx_transfer_mint_to_1 = system_transaction::transfer(
            &mint_keypair,
            &key1.pubkey(),
            genesis_config.rent.minimum_balance(0),
            genesis_config.hash(),
        );
        assert_eq!(bank0.process_transaction(&tx_transfer_mint_to_1), Ok(()));
        assert!(!bank0.is_empty());
    }

    #[test]
    fn test_bank_inherit_tx_count() {
        let (genesis_config, mint_keypair) = create_genesis_config(sol_to_lamports(1.0));
        let bank0 = Arc::new(Bank::new_for_tests(&genesis_config));

        // Bank 1
        let bank1 = Arc::new(Bank::new_from_parent(
            &bank0,
            &solana_sdk::pubkey::new_rand(),
            1,
        ));
        // Bank 2
        let bank2 = Bank::new_from_parent(&bank0, &solana_sdk::pubkey::new_rand(), 2);

        // transfer a token
        assert_eq!(
            bank1.process_transaction(&system_transaction::transfer(
                &mint_keypair,
                &Keypair::new().pubkey(),
                genesis_config.rent.minimum_balance(0),
                genesis_config.hash(),
            )),
            Ok(())
        );

        assert_eq!(bank0.transaction_count(), 0);
        assert_eq!(bank2.transaction_count(), 0);
        assert_eq!(bank1.transaction_count(), 1);

        bank1.squash();

        assert_eq!(bank0.transaction_count(), 0);
        assert_eq!(bank2.transaction_count(), 0);
        assert_eq!(bank1.transaction_count(), 1);

        let bank6 = Bank::new_from_parent(&bank1, &solana_sdk::pubkey::new_rand(), 3);
        assert_eq!(bank1.transaction_count(), 1);
        assert_eq!(bank6.transaction_count(), 1);

        bank6.squash();
        assert_eq!(bank6.transaction_count(), 1);
    }

    #[test]
    fn test_bank_inherit_fee_rate_governor() {
        let (mut genesis_config, _mint_keypair) = create_genesis_config(500);
        genesis_config
            .fee_rate_governor
            .target_lamports_per_signature = 123;

        let bank0 = Arc::new(Bank::new_for_tests(&genesis_config));
        let bank1 = Arc::new(new_from_parent(&bank0));
        assert_eq!(
            bank0.fee_rate_governor.target_lamports_per_signature / 2,
            bank1
                .fee_rate_governor
                .create_fee_calculator()
                .lamports_per_signature
        );
    }

    #[test]
    fn test_bank_vote_accounts() {
        let GenesisConfigInfo {
            genesis_config,
            mint_keypair,
            ..
        } = create_genesis_config_with_leader(500, &solana_sdk::pubkey::new_rand(), 1);
        let bank = Arc::new(Bank::new_for_tests(&genesis_config));

        let vote_accounts = bank.vote_accounts();
        assert_eq!(vote_accounts.len(), 1); // bootstrap validator has
                                            // to have a vote account

        let vote_keypair = Keypair::new();
        let instructions = vote_instruction::create_account(
            &mint_keypair.pubkey(),
            &vote_keypair.pubkey(),
            &VoteInit {
                node_pubkey: mint_keypair.pubkey(),
                authorized_voter: vote_keypair.pubkey(),
                authorized_withdrawer: vote_keypair.pubkey(),
                commission: 0,
            },
            10,
        );

        let message = Message::new(&instructions, Some(&mint_keypair.pubkey()));
        let transaction = Transaction::new(
            &[&mint_keypair, &vote_keypair],
            message,
            bank.last_blockhash(),
        );

        bank.process_transaction(&transaction).unwrap();

        let vote_accounts = bank.vote_accounts();

        assert_eq!(vote_accounts.len(), 2);

        assert!(vote_accounts.get(&vote_keypair.pubkey()).is_some());

        assert!(bank.withdraw(&vote_keypair.pubkey(), 10).is_ok());

        let vote_accounts = bank.vote_accounts();

        assert_eq!(vote_accounts.len(), 1);
    }

    #[test]
    fn test_bank_cloned_stake_delegations() {
        let GenesisConfigInfo {
            mut genesis_config,
            mint_keypair,
            ..
        } = create_genesis_config_with_leader(
            123_456_000_000_000,
            &solana_sdk::pubkey::new_rand(),
            123_000_000_000,
        );
        genesis_config.rent = Rent::default();
        let bank = Arc::new(Bank::new_for_tests(&genesis_config));

        let stake_delegations = bank.stakes_cache.stakes().stake_delegations().clone();
        assert_eq!(stake_delegations.len(), 1); // bootstrap validator has
                                                // to have a stake delegation

        let (vote_balance, stake_balance) = {
            let rent = &bank.rent_collector().rent;
            let vote_rent_exempt_reserve = rent.minimum_balance(VoteState::size_of());
            let stake_rent_exempt_reserve = rent.minimum_balance(StakeState::size_of());
            let minimum_delegation =
                solana_stake_program::get_minimum_delegation(&bank.feature_set);
            (
                vote_rent_exempt_reserve,
                stake_rent_exempt_reserve + minimum_delegation,
            )
        };

        let vote_keypair = Keypair::new();
        let mut instructions = vote_instruction::create_account(
            &mint_keypair.pubkey(),
            &vote_keypair.pubkey(),
            &VoteInit {
                node_pubkey: mint_keypair.pubkey(),
                authorized_voter: vote_keypair.pubkey(),
                authorized_withdrawer: vote_keypair.pubkey(),
                commission: 0,
            },
            vote_balance,
        );

        let stake_keypair = Keypair::new();
        instructions.extend(stake_instruction::create_account_and_delegate_stake(
            &mint_keypair.pubkey(),
            &stake_keypair.pubkey(),
            &vote_keypair.pubkey(),
            &Authorized::auto(&stake_keypair.pubkey()),
            &Lockup::default(),
            stake_balance,
        ));

        let message = Message::new(&instructions, Some(&mint_keypair.pubkey()));
        let transaction = Transaction::new(
            &[&mint_keypair, &vote_keypair, &stake_keypair],
            message,
            bank.last_blockhash(),
        );

        bank.process_transaction(&transaction).unwrap();

        let stake_delegations = bank.stakes_cache.stakes().stake_delegations().clone();
        assert_eq!(stake_delegations.len(), 2);
        assert!(stake_delegations.get(&stake_keypair.pubkey()).is_some());
    }

    #[allow(deprecated)]
    #[test]
    fn test_bank_fees_account() {
        let (mut genesis_config, _) = create_genesis_config(500);
        genesis_config.fee_rate_governor = FeeRateGovernor::new(12345, 0);
        let bank = Arc::new(Bank::new_for_tests(&genesis_config));

        let fees_account = bank.get_account(&sysvar::fees::id()).unwrap();
        let fees = from_account::<Fees, _>(&fees_account).unwrap();
        assert_eq!(
            bank.fee_rate_governor.lamports_per_signature,
            fees.fee_calculator.lamports_per_signature
        );
        assert_eq!(fees.fee_calculator.lamports_per_signature, 12345);
    }

    #[test]
    fn test_is_delta_with_no_committables() {
        let (genesis_config, mint_keypair) = create_genesis_config(8000);
        let bank = Bank::new_for_tests(&genesis_config);
        bank.is_delta.store(false, Relaxed);

        let keypair1 = Keypair::new();
        let keypair2 = Keypair::new();
        let fail_tx =
            system_transaction::transfer(&keypair1, &keypair2.pubkey(), 1, bank.last_blockhash());

        // Should fail with TransactionError::AccountNotFound, which means
        // the account which this tx operated on will not be committed. Thus
        // the bank is_delta should still be false
        assert_eq!(
            bank.process_transaction(&fail_tx),
            Err(TransactionError::AccountNotFound)
        );

        // Check the bank is_delta is still false
        assert!(!bank.is_delta.load(Relaxed));

        // Should fail with InstructionError, but InstructionErrors are committable,
        // so is_delta should be true
        assert_eq!(
            bank.transfer(10_001, &mint_keypair, &solana_sdk::pubkey::new_rand()),
            Err(TransactionError::InstructionError(
                0,
                SystemError::ResultWithNegativeLamports.into(),
            ))
        );

        assert!(bank.is_delta.load(Relaxed));
    }

    #[test]
    fn test_bank_get_program_accounts() {
        let (genesis_config, mint_keypair) = create_genesis_config(500);
        let parent = Arc::new(Bank::new_for_tests(&genesis_config));
        parent.restore_old_behavior_for_fragile_tests();

        let genesis_accounts: Vec<_> = parent.get_all_accounts_with_modified_slots().unwrap();
        assert!(
            genesis_accounts
                .iter()
                .any(|(pubkey, _, _)| *pubkey == mint_keypair.pubkey()),
            "mint pubkey not found"
        );
        assert!(
            genesis_accounts
                .iter()
                .any(|(pubkey, _, _)| solana_sdk::sysvar::is_sysvar_id(pubkey)),
            "no sysvars found"
        );

        let bank0 = Arc::new(new_from_parent(&parent));
        let pubkey0 = solana_sdk::pubkey::new_rand();
        let program_id = Pubkey::new(&[2; 32]);
        let account0 = AccountSharedData::new(1, 0, &program_id);
        bank0.store_account(&pubkey0, &account0);

        assert_eq!(
            bank0.get_program_accounts_modified_since_parent(&program_id),
            vec![(pubkey0, account0.clone())]
        );

        let bank1 = Arc::new(new_from_parent(&bank0));
        bank1.squash();
        assert_eq!(
            bank0
                .get_program_accounts(&program_id, &ScanConfig::default(),)
                .unwrap(),
            vec![(pubkey0, account0.clone())]
        );
        assert_eq!(
            bank1
                .get_program_accounts(&program_id, &ScanConfig::default(),)
                .unwrap(),
            vec![(pubkey0, account0)]
        );
        assert_eq!(
            bank1.get_program_accounts_modified_since_parent(&program_id),
            vec![]
        );

        let bank2 = Arc::new(new_from_parent(&bank1));
        let pubkey1 = solana_sdk::pubkey::new_rand();
        let account1 = AccountSharedData::new(3, 0, &program_id);
        bank2.store_account(&pubkey1, &account1);
        // Accounts with 0 lamports should be filtered out by Accounts::load_by_program()
        let pubkey2 = solana_sdk::pubkey::new_rand();
        let account2 = AccountSharedData::new(0, 0, &program_id);
        bank2.store_account(&pubkey2, &account2);

        let bank3 = Arc::new(new_from_parent(&bank2));
        bank3.squash();
        assert_eq!(
            bank1
                .get_program_accounts(&program_id, &ScanConfig::default(),)
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            bank3
                .get_program_accounts(&program_id, &ScanConfig::default(),)
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn test_get_filtered_indexed_accounts_limit_exceeded() {
        let (genesis_config, _mint_keypair) = create_genesis_config(500);
        let mut account_indexes = AccountSecondaryIndexes::default();
        account_indexes.indexes.insert(AccountIndex::ProgramId);
        let bank = Arc::new(Bank::new_with_config_for_tests(
            &genesis_config,
            account_indexes,
            false,
            AccountShrinkThreshold::default(),
        ));

        let address = Pubkey::new_unique();
        let program_id = Pubkey::new_unique();
        let limit = 100;
        let account = AccountSharedData::new(1, limit, &program_id);
        bank.store_account(&address, &account);

        assert!(bank
            .get_filtered_indexed_accounts(
                &IndexKey::ProgramId(program_id),
                |_| true,
                &ScanConfig::default(),
                Some(limit), // limit here will be exceeded, resulting in aborted scan
            )
            .is_err());
    }

    #[test]
    fn test_get_filtered_indexed_accounts() {
        let (genesis_config, _mint_keypair) = create_genesis_config(500);
        let mut account_indexes = AccountSecondaryIndexes::default();
        account_indexes.indexes.insert(AccountIndex::ProgramId);
        let bank = Arc::new(Bank::new_with_config_for_tests(
            &genesis_config,
            account_indexes,
            false,
            AccountShrinkThreshold::default(),
        ));

        let address = Pubkey::new_unique();
        let program_id = Pubkey::new_unique();
        let account = AccountSharedData::new(1, 0, &program_id);
        bank.store_account(&address, &account);

        let indexed_accounts = bank
            .get_filtered_indexed_accounts(
                &IndexKey::ProgramId(program_id),
                |_| true,
                &ScanConfig::default(),
                None,
            )
            .unwrap();
        assert_eq!(indexed_accounts.len(), 1);
        assert_eq!(indexed_accounts[0], (address, account));

        // Even though the account is re-stored in the bank (and the index) under a new program id,
        // it is still present in the index under the original program id as well. This
        // demonstrates the need for a redundant post-processing filter.
        let another_program_id = Pubkey::new_unique();
        let new_account = AccountSharedData::new(1, 0, &another_program_id);
        let bank = Arc::new(new_from_parent(&bank));
        bank.store_account(&address, &new_account);
        let indexed_accounts = bank
            .get_filtered_indexed_accounts(
                &IndexKey::ProgramId(program_id),
                |_| true,
                &ScanConfig::default(),
                None,
            )
            .unwrap();
        assert_eq!(indexed_accounts.len(), 1);
        assert_eq!(indexed_accounts[0], (address, new_account.clone()));
        let indexed_accounts = bank
            .get_filtered_indexed_accounts(
                &IndexKey::ProgramId(another_program_id),
                |_| true,
                &ScanConfig::default(),
                None,
            )
            .unwrap();
        assert_eq!(indexed_accounts.len(), 1);
        assert_eq!(indexed_accounts[0], (address, new_account.clone()));

        // Post-processing filter
        let indexed_accounts = bank
            .get_filtered_indexed_accounts(
                &IndexKey::ProgramId(program_id),
                |account| account.owner() == &program_id,
                &ScanConfig::default(),
                None,
            )
            .unwrap();
        assert!(indexed_accounts.is_empty());
        let indexed_accounts = bank
            .get_filtered_indexed_accounts(
                &IndexKey::ProgramId(another_program_id),
                |account| account.owner() == &another_program_id,
                &ScanConfig::default(),
                None,
            )
            .unwrap();
        assert_eq!(indexed_accounts.len(), 1);
        assert_eq!(indexed_accounts[0], (address, new_account));
    }

    #[test]
    fn test_status_cache_ancestors() {
        solana_logger::setup();
        let parent = create_simple_test_arc_bank(500);
        let bank1 = Arc::new(new_from_parent(&parent));
        let mut bank = bank1;
        for _ in 0..MAX_CACHE_ENTRIES * 2 {
            bank = Arc::new(new_from_parent(&bank));
            bank.squash();
        }

        let bank = new_from_parent(&bank);
        assert_eq!(
            bank.status_cache_ancestors(),
            (bank.slot() - MAX_CACHE_ENTRIES as u64..=bank.slot()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_add_builtin() {
        let (genesis_config, mint_keypair) = create_genesis_config(500);
        let mut bank = Bank::new_for_tests(&genesis_config);

        fn mock_vote_program_id() -> Pubkey {
            Pubkey::new(&[42u8; 32])
        }
        fn mock_vote_processor(
            _first_instruction_account: IndexOfAccount,
            invoke_context: &mut InvokeContext,
        ) -> std::result::Result<(), InstructionError> {
            let transaction_context = &invoke_context.transaction_context;
            let instruction_context = transaction_context.get_current_instruction_context()?;
            let program_id = instruction_context.get_last_program_key(transaction_context)?;
            if mock_vote_program_id() != *program_id {
                return Err(InstructionError::IncorrectProgramId);
            }
            Err(InstructionError::Custom(42))
        }

        assert!(bank.get_account(&mock_vote_program_id()).is_none());
        bank.add_builtin(
            "mock_vote_program",
            &mock_vote_program_id(),
            mock_vote_processor,
        );
        assert!(bank.get_account(&mock_vote_program_id()).is_some());

        let mock_account = Keypair::new();
        let mock_validator_identity = Keypair::new();
        let mut instructions = vote_instruction::create_account(
            &mint_keypair.pubkey(),
            &mock_account.pubkey(),
            &VoteInit {
                node_pubkey: mock_validator_identity.pubkey(),
                ..VoteInit::default()
            },
            1,
        );
        instructions[1].program_id = mock_vote_program_id();

        let message = Message::new(&instructions, Some(&mint_keypair.pubkey()));
        let transaction = Transaction::new(
            &[&mint_keypair, &mock_account, &mock_validator_identity],
            message,
            bank.last_blockhash(),
        );

        assert_eq!(
            bank.process_transaction(&transaction),
            Err(TransactionError::InstructionError(
                1,
                InstructionError::Custom(42)
            ))
        );
    }

    #[test]
    fn test_add_duplicate_static_program() {
        let GenesisConfigInfo {
            genesis_config,
            mint_keypair,
            ..
        } = create_genesis_config_with_leader(500, &solana_sdk::pubkey::new_rand(), 0);
        let mut bank = Bank::new_for_tests(&genesis_config);

        fn mock_vote_processor(
            _first_instruction_account: IndexOfAccount,
            _invoke_context: &mut InvokeContext,
        ) -> std::result::Result<(), InstructionError> {
            Err(InstructionError::Custom(42))
        }

        let mock_account = Keypair::new();
        let mock_validator_identity = Keypair::new();
        let instructions = vote_instruction::create_account(
            &mint_keypair.pubkey(),
            &mock_account.pubkey(),
            &VoteInit {
                node_pubkey: mock_validator_identity.pubkey(),
                ..VoteInit::default()
            },
            1,
        );

        let message = Message::new(&instructions, Some(&mint_keypair.pubkey()));
        let transaction = Transaction::new(
            &[&mint_keypair, &mock_account, &mock_validator_identity],
            message,
            bank.last_blockhash(),
        );

        let vote_loader_account = bank.get_account(&solana_vote_program::id()).unwrap();
        bank.add_builtin(
            "solana_vote_program",
            &solana_vote_program::id(),
            mock_vote_processor,
        );
        let new_vote_loader_account = bank.get_account(&solana_vote_program::id()).unwrap();
        // Vote loader account should not be updated since it was included in the genesis config.
        assert_eq!(vote_loader_account.data(), new_vote_loader_account.data());
        assert_eq!(
            bank.process_transaction(&transaction),
            Err(TransactionError::InstructionError(
                1,
                InstructionError::Custom(42)
            ))
        );
    }

    #[test]
    fn test_add_instruction_processor_for_existing_unrelated_accounts() {
        let mut bank = create_simple_test_bank(500);

        fn mock_ix_processor(
            _first_instruction_account: IndexOfAccount,
            _invoke_context: &mut InvokeContext,
        ) -> std::result::Result<(), InstructionError> {
            Err(InstructionError::Custom(42))
        }

        // Non-builtin loader accounts can not be used for instruction processing
        {
            let stakes = bank.stakes_cache.stakes();
            assert!(stakes.vote_accounts().as_ref().is_empty());
        }
        assert!(bank.stakes_cache.stakes().stake_delegations().is_empty());
        assert_eq!(bank.calculate_capitalization(true), bank.capitalization());

        let ((vote_id, vote_account), (stake_id, stake_account)) =
            crate::stakes::tests::create_staked_node_accounts(1_0000);
        bank.capitalization
            .fetch_add(vote_account.lamports() + stake_account.lamports(), Relaxed);
        bank.store_account(&vote_id, &vote_account);
        bank.store_account(&stake_id, &stake_account);
        {
            let stakes = bank.stakes_cache.stakes();
            assert!(!stakes.vote_accounts().as_ref().is_empty());
        }
        assert!(!bank.stakes_cache.stakes().stake_delegations().is_empty());
        assert_eq!(bank.calculate_capitalization(true), bank.capitalization());

        bank.add_builtin("mock_program1", &vote_id, mock_ix_processor);
        bank.add_builtin("mock_program2", &stake_id, mock_ix_processor);
        {
            let stakes = bank.stakes_cache.stakes();
            assert!(stakes.vote_accounts().as_ref().is_empty());
        }
        assert!(bank.stakes_cache.stakes().stake_delegations().is_empty());
        assert_eq!(bank.calculate_capitalization(true), bank.capitalization());
        assert_eq!(
            "mock_program1",
            String::from_utf8_lossy(bank.get_account(&vote_id).unwrap_or_default().data())
        );
        assert_eq!(
            "mock_program2",
            String::from_utf8_lossy(bank.get_account(&stake_id).unwrap_or_default().data())
        );

        // Re-adding builtin programs should be no-op
        bank.update_accounts_hash();
        let old_hash = bank.get_accounts_hash();
        bank.add_builtin("mock_program1", &vote_id, mock_ix_processor);
        bank.add_builtin("mock_program2", &stake_id, mock_ix_processor);
        bank.update_accounts_hash();
        let new_hash = bank.get_accounts_hash();
        assert_eq!(old_hash, new_hash);
        {
            let stakes = bank.stakes_cache.stakes();
            assert!(stakes.vote_accounts().as_ref().is_empty());
        }
        assert!(bank.stakes_cache.stakes().stake_delegations().is_empty());
        assert_eq!(bank.calculate_capitalization(true), bank.capitalization());
        assert_eq!(
            "mock_program1",
            String::from_utf8_lossy(bank.get_account(&vote_id).unwrap_or_default().data())
        );
        assert_eq!(
            "mock_program2",
            String::from_utf8_lossy(bank.get_account(&stake_id).unwrap_or_default().data())
        );
    }

    #[allow(deprecated)]
    #[test]
    fn test_recent_blockhashes_sysvar() {
        let mut bank = create_simple_test_arc_bank(500);
        for i in 1..5 {
            let bhq_account = bank.get_account(&sysvar::recent_blockhashes::id()).unwrap();
            let recent_blockhashes =
                from_account::<sysvar::recent_blockhashes::RecentBlockhashes, _>(&bhq_account)
                    .unwrap();
            // Check length
            assert_eq!(recent_blockhashes.len(), i);
            let most_recent_hash = recent_blockhashes.iter().next().unwrap().blockhash;
            // Check order
            assert!(bank.is_hash_valid_for_age(&most_recent_hash, 0));
            goto_end_of_slot(Arc::get_mut(&mut bank).unwrap());
            bank = Arc::new(new_from_parent(&bank));
        }
    }

    #[allow(deprecated)]
    #[test]
    fn test_blockhash_queue_sysvar_consistency() {
        let mut bank = create_simple_test_arc_bank(100_000);
        goto_end_of_slot(Arc::get_mut(&mut bank).unwrap());

        let bhq_account = bank.get_account(&sysvar::recent_blockhashes::id()).unwrap();
        let recent_blockhashes =
            from_account::<sysvar::recent_blockhashes::RecentBlockhashes, _>(&bhq_account).unwrap();

        let sysvar_recent_blockhash = recent_blockhashes[0].blockhash;
        let bank_last_blockhash = bank.last_blockhash();
        assert_eq!(sysvar_recent_blockhash, bank_last_blockhash);
    }

    #[test]
    fn test_hash_internal_state_unchanged() {
        let (genesis_config, _) = create_genesis_config(500);
        let bank0 = Arc::new(Bank::new_for_tests(&genesis_config));
        bank0.freeze();
        let bank0_hash = bank0.hash();
        let bank1 = Bank::new_from_parent(&bank0, &Pubkey::default(), 1);
        bank1.freeze();
        let bank1_hash = bank1.hash();
        // Checkpointing should always result in a new state
        assert_ne!(bank0_hash, bank1_hash);
    }

    #[test]
    fn test_ticks_change_state() {
        let (genesis_config, _) = create_genesis_config(500);
        let bank = Arc::new(Bank::new_for_tests(&genesis_config));
        let bank1 = new_from_parent(&bank);
        let hash1 = bank1.hash_internal_state();
        // ticks don't change its state unless a block boundary is crossed
        for _ in 0..genesis_config.ticks_per_slot {
            assert_eq!(bank1.hash_internal_state(), hash1);
            bank1.register_tick(&Hash::default());
        }
        assert_ne!(bank1.hash_internal_state(), hash1);
    }

    #[ignore]
    #[test]
    fn test_banks_leak() {
        fn add_lotsa_stake_accounts(genesis_config: &mut GenesisConfig) {
            const LOTSA: usize = 4_096;

            (0..LOTSA).for_each(|_| {
                let pubkey = solana_sdk::pubkey::new_rand();
                genesis_config.add_account(
                    pubkey,
                    stake_state::create_lockup_stake_account(
                        &Authorized::auto(&pubkey),
                        &Lockup::default(),
                        &Rent::default(),
                        50_000_000,
                    ),
                );
            });
        }
        solana_logger::setup();
        let (mut genesis_config, _) = create_genesis_config(100_000_000_000_000);
        add_lotsa_stake_accounts(&mut genesis_config);
        let mut bank = std::sync::Arc::new(Bank::new_for_tests(&genesis_config));
        let mut num_banks = 0;
        let pid = std::process::id();
        #[cfg(not(target_os = "linux"))]
        error!(
            "\nYou can run this to watch RAM:\n   while read -p 'banks: '; do echo $(( $(ps -o vsize= -p {})/$REPLY));done", pid
        );
        loop {
            num_banks += 1;
            bank = std::sync::Arc::new(new_from_parent(&bank));
            if num_banks % 100 == 0 {
                #[cfg(target_os = "linux")]
                {
                    let pages_consumed = std::fs::read_to_string(format!("/proc/{}/statm", pid))
                        .unwrap()
                        .split_whitespace()
                        .next()
                        .unwrap()
                        .parse::<usize>()
                        .unwrap();
                    error!(
                        "at {} banks: {} mem or {}kB/bank",
                        num_banks,
                        pages_consumed * 4096,
                        (pages_consumed * 4) / num_banks
                    );
                }
                #[cfg(not(target_os = "linux"))]
                {
                    error!("{} banks, sleeping for 5 sec", num_banks);
                    std::thread::sleep(Duration::from_secs(5));
                }
            }
        }
    }

    fn get_nonce_blockhash(bank: &Bank, nonce_pubkey: &Pubkey) -> Option<Hash> {
        let account = bank.get_account(nonce_pubkey)?;
        let nonce_versions = StateMut::<nonce::state::Versions>::state(&account);
        match nonce_versions.ok()?.state() {
            nonce::State::Initialized(ref data) => Some(data.blockhash()),
            _ => None,
        }
    }

    fn nonce_setup(
        bank: &mut Arc<Bank>,
        mint_keypair: &Keypair,
        custodian_lamports: u64,
        nonce_lamports: u64,
        nonce_authority: Option<Pubkey>,
    ) -> Result<(Keypair, Keypair)> {
        let custodian_keypair = Keypair::new();
        let nonce_keypair = Keypair::new();
        /* Setup accounts */
        let mut setup_ixs = vec![system_instruction::transfer(
            &mint_keypair.pubkey(),
            &custodian_keypair.pubkey(),
            custodian_lamports,
        )];
        let nonce_authority = nonce_authority.unwrap_or_else(|| nonce_keypair.pubkey());
        setup_ixs.extend_from_slice(&system_instruction::create_nonce_account(
            &custodian_keypair.pubkey(),
            &nonce_keypair.pubkey(),
            &nonce_authority,
            nonce_lamports,
        ));
        let message = Message::new(&setup_ixs, Some(&mint_keypair.pubkey()));
        let setup_tx = Transaction::new(
            &[mint_keypair, &custodian_keypair, &nonce_keypair],
            message,
            bank.last_blockhash(),
        );
        bank.process_transaction(&setup_tx)?;
        Ok((custodian_keypair, nonce_keypair))
    }

    fn setup_nonce_with_bank<F>(
        supply_lamports: u64,
        mut genesis_cfg_fn: F,
        custodian_lamports: u64,
        nonce_lamports: u64,
        nonce_authority: Option<Pubkey>,
        feature_set: FeatureSet,
    ) -> Result<(Arc<Bank>, Keypair, Keypair, Keypair)>
    where
        F: FnMut(&mut GenesisConfig),
    {
        let (mut genesis_config, mint_keypair) = create_genesis_config(supply_lamports);
        genesis_config.rent.lamports_per_byte_year = 0;
        genesis_cfg_fn(&mut genesis_config);
        let mut bank = Bank::new_for_tests(&genesis_config);
        bank.feature_set = Arc::new(feature_set);
        let mut bank = Arc::new(bank);

        // Banks 0 and 1 have no fees, wait two blocks before
        // initializing our nonce accounts
        for _ in 0..2 {
            goto_end_of_slot(Arc::get_mut(&mut bank).unwrap());
            bank = Arc::new(new_from_parent(&bank));
        }

        let (custodian_keypair, nonce_keypair) = nonce_setup(
            &mut bank,
            &mint_keypair,
            custodian_lamports,
            nonce_lamports,
            nonce_authority,
        )?;

        // The setup nonce is not valid to be used until the next bank
        // so wait one more block
        goto_end_of_slot(Arc::get_mut(&mut bank).unwrap());
        bank = Arc::new(new_from_parent(&bank));

        Ok((bank, mint_keypair, custodian_keypair, nonce_keypair))
    }

    impl Bank {
        fn next_durable_nonce(&self) -> DurableNonce {
            let hash_queue = self.blockhash_queue.read().unwrap();
            let last_blockhash = hash_queue.last_hash();
            DurableNonce::from_blockhash(&last_blockhash)
        }
    }

    #[test]
    fn test_check_transaction_for_nonce_ok() {
        let (bank, _mint_keypair, custodian_keypair, nonce_keypair) = setup_nonce_with_bank(
            10_000_000,
            |_| {},
            5_000_000,
            250_000,
            None,
            FeatureSet::all_enabled(),
        )
        .unwrap();
        let custodian_pubkey = custodian_keypair.pubkey();
        let nonce_pubkey = nonce_keypair.pubkey();

        let nonce_hash = get_nonce_blockhash(&bank, &nonce_pubkey).unwrap();
        let tx = Transaction::new_signed_with_payer(
            &[
                system_instruction::advance_nonce_account(&nonce_pubkey, &nonce_pubkey),
                system_instruction::transfer(&custodian_pubkey, &nonce_pubkey, 100_000),
            ],
            Some(&custodian_pubkey),
            &[&custodian_keypair, &nonce_keypair],
            nonce_hash,
        );
        let nonce_account = bank.get_account(&nonce_pubkey).unwrap();
        assert_eq!(
            bank.check_transaction_for_nonce(
                &SanitizedTransaction::from_transaction_for_tests(tx),
                &bank.next_durable_nonce(),
            ),
            Some((nonce_pubkey, nonce_account))
        );
    }

    #[test]
    fn test_check_transaction_for_nonce_not_nonce_fail() {
        let (bank, _mint_keypair, custodian_keypair, nonce_keypair) = setup_nonce_with_bank(
            10_000_000,
            |_| {},
            5_000_000,
            250_000,
            None,
            FeatureSet::all_enabled(),
        )
        .unwrap();
        let custodian_pubkey = custodian_keypair.pubkey();
        let nonce_pubkey = nonce_keypair.pubkey();

        let nonce_hash = get_nonce_blockhash(&bank, &nonce_pubkey).unwrap();
        let tx = Transaction::new_signed_with_payer(
            &[
                system_instruction::transfer(&custodian_pubkey, &nonce_pubkey, 100_000),
                system_instruction::advance_nonce_account(&nonce_pubkey, &nonce_pubkey),
            ],
            Some(&custodian_pubkey),
            &[&custodian_keypair, &nonce_keypair],
            nonce_hash,
        );
        assert!(bank
            .check_transaction_for_nonce(
                &SanitizedTransaction::from_transaction_for_tests(tx,),
                &bank.next_durable_nonce(),
            )
            .is_none());
    }

    #[test]
    fn test_check_transaction_for_nonce_missing_ix_pubkey_fail() {
        let (bank, _mint_keypair, custodian_keypair, nonce_keypair) = setup_nonce_with_bank(
            10_000_000,
            |_| {},
            5_000_000,
            250_000,
            None,
            FeatureSet::all_enabled(),
        )
        .unwrap();
        let custodian_pubkey = custodian_keypair.pubkey();
        let nonce_pubkey = nonce_keypair.pubkey();

        let nonce_hash = get_nonce_blockhash(&bank, &nonce_pubkey).unwrap();
        let mut tx = Transaction::new_signed_with_payer(
            &[
                system_instruction::advance_nonce_account(&nonce_pubkey, &nonce_pubkey),
                system_instruction::transfer(&custodian_pubkey, &nonce_pubkey, 100_000),
            ],
            Some(&custodian_pubkey),
            &[&custodian_keypair, &nonce_keypair],
            nonce_hash,
        );
        tx.message.instructions[0].accounts.clear();
        assert!(bank
            .check_transaction_for_nonce(
                &SanitizedTransaction::from_transaction_for_tests(tx),
                &bank.next_durable_nonce(),
            )
            .is_none());
    }

    #[test]
    fn test_check_transaction_for_nonce_nonce_acc_does_not_exist_fail() {
        let (bank, _mint_keypair, custodian_keypair, nonce_keypair) = setup_nonce_with_bank(
            10_000_000,
            |_| {},
            5_000_000,
            250_000,
            None,
            FeatureSet::all_enabled(),
        )
        .unwrap();
        let custodian_pubkey = custodian_keypair.pubkey();
        let nonce_pubkey = nonce_keypair.pubkey();
        let missing_keypair = Keypair::new();
        let missing_pubkey = missing_keypair.pubkey();

        let nonce_hash = get_nonce_blockhash(&bank, &nonce_pubkey).unwrap();
        let tx = Transaction::new_signed_with_payer(
            &[
                system_instruction::advance_nonce_account(&missing_pubkey, &nonce_pubkey),
                system_instruction::transfer(&custodian_pubkey, &nonce_pubkey, 100_000),
            ],
            Some(&custodian_pubkey),
            &[&custodian_keypair, &nonce_keypair],
            nonce_hash,
        );
        assert!(bank
            .check_transaction_for_nonce(
                &SanitizedTransaction::from_transaction_for_tests(tx),
                &bank.next_durable_nonce(),
            )
            .is_none());
    }

    #[test]
    fn test_check_transaction_for_nonce_bad_tx_hash_fail() {
        let (bank, _mint_keypair, custodian_keypair, nonce_keypair) = setup_nonce_with_bank(
            10_000_000,
            |_| {},
            5_000_000,
            250_000,
            None,
            FeatureSet::all_enabled(),
        )
        .unwrap();
        let custodian_pubkey = custodian_keypair.pubkey();
        let nonce_pubkey = nonce_keypair.pubkey();

        let tx = Transaction::new_signed_with_payer(
            &[
                system_instruction::advance_nonce_account(&nonce_pubkey, &nonce_pubkey),
                system_instruction::transfer(&custodian_pubkey, &nonce_pubkey, 100_000),
            ],
            Some(&custodian_pubkey),
            &[&custodian_keypair, &nonce_keypair],
            Hash::default(),
        );
        assert!(bank
            .check_transaction_for_nonce(
                &SanitizedTransaction::from_transaction_for_tests(tx),
                &bank.next_durable_nonce(),
            )
            .is_none());
    }

    #[test]
    fn test_assign_from_nonce_account_fail() {
        let bank = create_simple_test_arc_bank(100_000_000);
        let nonce = Keypair::new();
        let nonce_account = AccountSharedData::new_data(
            42_424_242,
            &nonce::state::Versions::new(nonce::State::Initialized(nonce::state::Data::default())),
            &system_program::id(),
        )
        .unwrap();
        let blockhash = bank.last_blockhash();
        bank.store_account(&nonce.pubkey(), &nonce_account);

        let ix = system_instruction::assign(&nonce.pubkey(), &Pubkey::new(&[9u8; 32]));
        let message = Message::new(&[ix], Some(&nonce.pubkey()));
        let tx = Transaction::new(&[&nonce], message, blockhash);

        let expect = Err(TransactionError::InstructionError(
            0,
            InstructionError::ModifiedProgramId,
        ));
        assert_eq!(bank.process_transaction(&tx), expect);
    }

    #[test]
    fn test_nonce_must_be_advanceable() {
        let mut bank = create_simple_test_bank(100_000_000);
        bank.feature_set = Arc::new(FeatureSet::all_enabled());
        let bank = Arc::new(bank);
        let nonce_keypair = Keypair::new();
        let nonce_authority = nonce_keypair.pubkey();
        let durable_nonce = DurableNonce::from_blockhash(&bank.last_blockhash());
        let nonce_account = AccountSharedData::new_data(
            42_424_242,
            &nonce::state::Versions::new(nonce::State::Initialized(nonce::state::Data::new(
                nonce_authority,
                durable_nonce,
                5000,
            ))),
            &system_program::id(),
        )
        .unwrap();
        bank.store_account(&nonce_keypair.pubkey(), &nonce_account);

        let ix =
            system_instruction::advance_nonce_account(&nonce_keypair.pubkey(), &nonce_authority);
        let message = Message::new(&[ix], Some(&nonce_keypair.pubkey()));
        let tx = Transaction::new(&[&nonce_keypair], message, *durable_nonce.as_hash());
        assert_eq!(
            bank.process_transaction(&tx),
            Err(TransactionError::BlockhashNotFound)
        );
    }

    #[test]
    fn test_nonce_transaction() {
        let (mut bank, _mint_keypair, custodian_keypair, nonce_keypair) = setup_nonce_with_bank(
            10_000_000,
            |_| {},
            5_000_000,
            250_000,
            None,
            FeatureSet::all_enabled(),
        )
        .unwrap();
        let alice_keypair = Keypair::new();
        let alice_pubkey = alice_keypair.pubkey();
        let custodian_pubkey = custodian_keypair.pubkey();
        let nonce_pubkey = nonce_keypair.pubkey();

        assert_eq!(bank.get_balance(&custodian_pubkey), 4_750_000);
        assert_eq!(bank.get_balance(&nonce_pubkey), 250_000);

        /* Grab the hash stored in the nonce account */
        let nonce_hash = get_nonce_blockhash(&bank, &nonce_pubkey).unwrap();

        /* Kick nonce hash off the blockhash_queue */
        for _ in 0..MAX_RECENT_BLOCKHASHES + 1 {
            goto_end_of_slot(Arc::get_mut(&mut bank).unwrap());
            bank = Arc::new(new_from_parent(&bank));
        }

        /* Expect a non-Nonce transfer to fail */
        assert_eq!(
            bank.process_transaction(&system_transaction::transfer(
                &custodian_keypair,
                &alice_pubkey,
                100_000,
                nonce_hash
            ),),
            Err(TransactionError::BlockhashNotFound),
        );
        /* Check fee not charged */
        assert_eq!(bank.get_balance(&custodian_pubkey), 4_750_000);

        /* Nonce transfer */
        let nonce_tx = Transaction::new_signed_with_payer(
            &[
                system_instruction::advance_nonce_account(&nonce_pubkey, &nonce_pubkey),
                system_instruction::transfer(&custodian_pubkey, &alice_pubkey, 100_000),
            ],
            Some(&custodian_pubkey),
            &[&custodian_keypair, &nonce_keypair],
            nonce_hash,
        );
        assert_eq!(bank.process_transaction(&nonce_tx), Ok(()));

        /* Check balances */
        let mut recent_message = nonce_tx.message;
        recent_message.recent_blockhash = bank.last_blockhash();
        let mut expected_balance = 4_650_000
            - bank
                .get_fee_for_message(&recent_message.try_into().unwrap())
                .unwrap();
        assert_eq!(bank.get_balance(&custodian_pubkey), expected_balance);
        assert_eq!(bank.get_balance(&nonce_pubkey), 250_000);
        assert_eq!(bank.get_balance(&alice_pubkey), 100_000);

        /* Confirm stored nonce has advanced */
        let new_nonce = get_nonce_blockhash(&bank, &nonce_pubkey).unwrap();
        assert_ne!(nonce_hash, new_nonce);

        /* Nonce re-use fails */
        let nonce_tx = Transaction::new_signed_with_payer(
            &[
                system_instruction::advance_nonce_account(&nonce_pubkey, &nonce_pubkey),
                system_instruction::transfer(&custodian_pubkey, &alice_pubkey, 100_000),
            ],
            Some(&custodian_pubkey),
            &[&custodian_keypair, &nonce_keypair],
            nonce_hash,
        );
        assert_eq!(
            bank.process_transaction(&nonce_tx),
            Err(TransactionError::BlockhashNotFound)
        );
        /* Check fee not charged and nonce not advanced */
        assert_eq!(bank.get_balance(&custodian_pubkey), expected_balance);
        assert_eq!(
            new_nonce,
            get_nonce_blockhash(&bank, &nonce_pubkey).unwrap()
        );

        let nonce_hash = new_nonce;

        /* Kick nonce hash off the blockhash_queue */
        for _ in 0..MAX_RECENT_BLOCKHASHES + 1 {
            goto_end_of_slot(Arc::get_mut(&mut bank).unwrap());
            bank = Arc::new(new_from_parent(&bank));
        }

        let nonce_tx = Transaction::new_signed_with_payer(
            &[
                system_instruction::advance_nonce_account(&nonce_pubkey, &nonce_pubkey),
                system_instruction::transfer(&custodian_pubkey, &alice_pubkey, 100_000_000),
            ],
            Some(&custodian_pubkey),
            &[&custodian_keypair, &nonce_keypair],
            nonce_hash,
        );
        assert_eq!(
            bank.process_transaction(&nonce_tx),
            Err(TransactionError::InstructionError(
                1,
                system_instruction::SystemError::ResultWithNegativeLamports.into(),
            ))
        );
        /* Check fee charged and nonce has advanced */
        let mut recent_message = nonce_tx.message.clone();
        recent_message.recent_blockhash = bank.last_blockhash();
        expected_balance -= bank
            .get_fee_for_message(&SanitizedMessage::try_from(recent_message).unwrap())
            .unwrap();
        assert_eq!(bank.get_balance(&custodian_pubkey), expected_balance);
        assert_ne!(
            nonce_hash,
            get_nonce_blockhash(&bank, &nonce_pubkey).unwrap()
        );
        /* Confirm replaying a TX that failed with InstructionError::* now
         * fails with TransactionError::BlockhashNotFound
         */
        assert_eq!(
            bank.process_transaction(&nonce_tx),
            Err(TransactionError::BlockhashNotFound),
        );
    }

    #[test]
    fn test_nonce_transaction_with_tx_wide_caps() {
        let feature_set = FeatureSet::all_enabled();
        let (mut bank, _mint_keypair, custodian_keypair, nonce_keypair) =
            setup_nonce_with_bank(10_000_000, |_| {}, 5_000_000, 250_000, None, feature_set)
                .unwrap();
        let alice_keypair = Keypair::new();
        let alice_pubkey = alice_keypair.pubkey();
        let custodian_pubkey = custodian_keypair.pubkey();
        let nonce_pubkey = nonce_keypair.pubkey();

        assert_eq!(bank.get_balance(&custodian_pubkey), 4_750_000);
        assert_eq!(bank.get_balance(&nonce_pubkey), 250_000);

        /* Grab the hash stored in the nonce account */
        let nonce_hash = get_nonce_blockhash(&bank, &nonce_pubkey).unwrap();

        /* Kick nonce hash off the blockhash_queue */
        for _ in 0..MAX_RECENT_BLOCKHASHES + 1 {
            goto_end_of_slot(Arc::get_mut(&mut bank).unwrap());
            bank = Arc::new(new_from_parent(&bank));
        }

        /* Expect a non-Nonce transfer to fail */
        assert_eq!(
            bank.process_transaction(&system_transaction::transfer(
                &custodian_keypair,
                &alice_pubkey,
                100_000,
                nonce_hash
            ),),
            Err(TransactionError::BlockhashNotFound),
        );
        /* Check fee not charged */
        assert_eq!(bank.get_balance(&custodian_pubkey), 4_750_000);

        /* Nonce transfer */
        let nonce_tx = Transaction::new_signed_with_payer(
            &[
                system_instruction::advance_nonce_account(&nonce_pubkey, &nonce_pubkey),
                system_instruction::transfer(&custodian_pubkey, &alice_pubkey, 100_000),
            ],
            Some(&custodian_pubkey),
            &[&custodian_keypair, &nonce_keypair],
            nonce_hash,
        );
        assert_eq!(bank.process_transaction(&nonce_tx), Ok(()));

        /* Check balances */
        let mut recent_message = nonce_tx.message;
        recent_message.recent_blockhash = bank.last_blockhash();
        let mut expected_balance = 4_650_000
            - bank
                .get_fee_for_message(&recent_message.try_into().unwrap())
                .unwrap();
        assert_eq!(bank.get_balance(&custodian_pubkey), expected_balance);
        assert_eq!(bank.get_balance(&nonce_pubkey), 250_000);
        assert_eq!(bank.get_balance(&alice_pubkey), 100_000);

        /* Confirm stored nonce has advanced */
        let new_nonce = get_nonce_blockhash(&bank, &nonce_pubkey).unwrap();
        assert_ne!(nonce_hash, new_nonce);

        /* Nonce re-use fails */
        let nonce_tx = Transaction::new_signed_with_payer(
            &[
                system_instruction::advance_nonce_account(&nonce_pubkey, &nonce_pubkey),
                system_instruction::transfer(&custodian_pubkey, &alice_pubkey, 100_000),
            ],
            Some(&custodian_pubkey),
            &[&custodian_keypair, &nonce_keypair],
            nonce_hash,
        );
        assert_eq!(
            bank.process_transaction(&nonce_tx),
            Err(TransactionError::BlockhashNotFound)
        );
        /* Check fee not charged and nonce not advanced */
        assert_eq!(bank.get_balance(&custodian_pubkey), expected_balance);
        assert_eq!(
            new_nonce,
            get_nonce_blockhash(&bank, &nonce_pubkey).unwrap()
        );

        let nonce_hash = new_nonce;

        /* Kick nonce hash off the blockhash_queue */
        for _ in 0..MAX_RECENT_BLOCKHASHES + 1 {
            goto_end_of_slot(Arc::get_mut(&mut bank).unwrap());
            bank = Arc::new(new_from_parent(&bank));
        }

        let nonce_tx = Transaction::new_signed_with_payer(
            &[
                system_instruction::advance_nonce_account(&nonce_pubkey, &nonce_pubkey),
                system_instruction::transfer(&custodian_pubkey, &alice_pubkey, 100_000_000),
            ],
            Some(&custodian_pubkey),
            &[&custodian_keypair, &nonce_keypair],
            nonce_hash,
        );
        assert_eq!(
            bank.process_transaction(&nonce_tx),
            Err(TransactionError::InstructionError(
                1,
                system_instruction::SystemError::ResultWithNegativeLamports.into(),
            ))
        );
        /* Check fee charged and nonce has advanced */
        let mut recent_message = nonce_tx.message.clone();
        recent_message.recent_blockhash = bank.last_blockhash();
        expected_balance -= bank
            .get_fee_for_message(&SanitizedMessage::try_from(recent_message).unwrap())
            .unwrap();
        assert_eq!(bank.get_balance(&custodian_pubkey), expected_balance);
        assert_ne!(
            nonce_hash,
            get_nonce_blockhash(&bank, &nonce_pubkey).unwrap()
        );
        /* Confirm replaying a TX that failed with InstructionError::* now
         * fails with TransactionError::BlockhashNotFound
         */
        assert_eq!(
            bank.process_transaction(&nonce_tx),
            Err(TransactionError::BlockhashNotFound),
        );
    }

    #[test]
    fn test_nonce_authority() {
        solana_logger::setup();
        let (mut bank, _mint_keypair, custodian_keypair, nonce_keypair) = setup_nonce_with_bank(
            10_000_000,
            |_| {},
            5_000_000,
            250_000,
            None,
            FeatureSet::all_enabled(),
        )
        .unwrap();
        let alice_keypair = Keypair::new();
        let alice_pubkey = alice_keypair.pubkey();
        let custodian_pubkey = custodian_keypair.pubkey();
        let nonce_pubkey = nonce_keypair.pubkey();
        let bad_nonce_authority_keypair = Keypair::new();
        let bad_nonce_authority = bad_nonce_authority_keypair.pubkey();
        let custodian_account = bank.get_account(&custodian_pubkey).unwrap();

        debug!("alice: {}", alice_pubkey);
        debug!("custodian: {}", custodian_pubkey);
        debug!("nonce: {}", nonce_pubkey);
        debug!("nonce account: {:?}", bank.get_account(&nonce_pubkey));
        debug!("cust: {:?}", custodian_account);
        let nonce_hash = get_nonce_blockhash(&bank, &nonce_pubkey).unwrap();

        for _ in 0..MAX_RECENT_BLOCKHASHES + 1 {
            goto_end_of_slot(Arc::get_mut(&mut bank).unwrap());
            bank = Arc::new(new_from_parent(&bank));
        }

        let nonce_tx = Transaction::new_signed_with_payer(
            &[
                system_instruction::advance_nonce_account(&nonce_pubkey, &bad_nonce_authority),
                system_instruction::transfer(&custodian_pubkey, &alice_pubkey, 42),
            ],
            Some(&custodian_pubkey),
            &[&custodian_keypair, &bad_nonce_authority_keypair],
            nonce_hash,
        );
        debug!("{:?}", nonce_tx);
        let initial_custodian_balance = custodian_account.lamports();
        assert_eq!(
            bank.process_transaction(&nonce_tx),
            Err(TransactionError::BlockhashNotFound),
        );
        /* Check fee was *not* charged and nonce has *not* advanced */
        let mut recent_message = nonce_tx.message;
        recent_message.recent_blockhash = bank.last_blockhash();
        assert_eq!(
            bank.get_balance(&custodian_pubkey),
            initial_custodian_balance
        );
        assert_eq!(
            nonce_hash,
            get_nonce_blockhash(&bank, &nonce_pubkey).unwrap()
        );
    }

    #[test]
    fn test_nonce_payer() {
        solana_logger::setup();
        let nonce_starting_balance = 250_000;
        let (mut bank, _mint_keypair, custodian_keypair, nonce_keypair) = setup_nonce_with_bank(
            10_000_000,
            |_| {},
            5_000_000,
            nonce_starting_balance,
            None,
            FeatureSet::all_enabled(),
        )
        .unwrap();
        let alice_keypair = Keypair::new();
        let alice_pubkey = alice_keypair.pubkey();
        let custodian_pubkey = custodian_keypair.pubkey();
        let nonce_pubkey = nonce_keypair.pubkey();

        debug!("alice: {}", alice_pubkey);
        debug!("custodian: {}", custodian_pubkey);
        debug!("nonce: {}", nonce_pubkey);
        debug!("nonce account: {:?}", bank.get_account(&nonce_pubkey));
        debug!("cust: {:?}", bank.get_account(&custodian_pubkey));
        let nonce_hash = get_nonce_blockhash(&bank, &nonce_pubkey).unwrap();

        for _ in 0..MAX_RECENT_BLOCKHASHES + 1 {
            goto_end_of_slot(Arc::get_mut(&mut bank).unwrap());
            bank = Arc::new(new_from_parent(&bank));
        }

        let nonce_tx = Transaction::new_signed_with_payer(
            &[
                system_instruction::advance_nonce_account(&nonce_pubkey, &nonce_pubkey),
                system_instruction::transfer(&custodian_pubkey, &alice_pubkey, 100_000_000),
            ],
            Some(&nonce_pubkey),
            &[&custodian_keypair, &nonce_keypair],
            nonce_hash,
        );
        debug!("{:?}", nonce_tx);
        assert_eq!(
            bank.process_transaction(&nonce_tx),
            Err(TransactionError::InstructionError(
                1,
                system_instruction::SystemError::ResultWithNegativeLamports.into(),
            ))
        );
        /* Check fee charged and nonce has advanced */
        let mut recent_message = nonce_tx.message;
        recent_message.recent_blockhash = bank.last_blockhash();
        assert_eq!(
            bank.get_balance(&nonce_pubkey),
            nonce_starting_balance
                - bank
                    .get_fee_for_message(&recent_message.try_into().unwrap())
                    .unwrap()
        );
        assert_ne!(
            nonce_hash,
            get_nonce_blockhash(&bank, &nonce_pubkey).unwrap()
        );
    }

    #[test]
    fn test_nonce_payer_tx_wide_cap() {
        solana_logger::setup();
        let nonce_starting_balance =
            250_000 + FeeStructure::default().compute_fee_bins.last().unwrap().fee;
        let feature_set = FeatureSet::all_enabled();
        let (mut bank, _mint_keypair, custodian_keypair, nonce_keypair) = setup_nonce_with_bank(
            10_000_000,
            |_| {},
            5_000_000,
            nonce_starting_balance,
            None,
            feature_set,
        )
        .unwrap();
        let alice_keypair = Keypair::new();
        let alice_pubkey = alice_keypair.pubkey();
        let custodian_pubkey = custodian_keypair.pubkey();
        let nonce_pubkey = nonce_keypair.pubkey();

        debug!("alice: {}", alice_pubkey);
        debug!("custodian: {}", custodian_pubkey);
        debug!("nonce: {}", nonce_pubkey);
        debug!("nonce account: {:?}", bank.get_account(&nonce_pubkey));
        debug!("cust: {:?}", bank.get_account(&custodian_pubkey));
        let nonce_hash = get_nonce_blockhash(&bank, &nonce_pubkey).unwrap();

        for _ in 0..MAX_RECENT_BLOCKHASHES + 1 {
            goto_end_of_slot(Arc::get_mut(&mut bank).unwrap());
            bank = Arc::new(new_from_parent(&bank));
        }

        let nonce_tx = Transaction::new_signed_with_payer(
            &[
                system_instruction::advance_nonce_account(&nonce_pubkey, &nonce_pubkey),
                system_instruction::transfer(&custodian_pubkey, &alice_pubkey, 100_000_000),
            ],
            Some(&nonce_pubkey),
            &[&custodian_keypair, &nonce_keypair],
            nonce_hash,
        );
        debug!("{:?}", nonce_tx);

        assert_eq!(
            bank.process_transaction(&nonce_tx),
            Err(TransactionError::InstructionError(
                1,
                system_instruction::SystemError::ResultWithNegativeLamports.into(),
            ))
        );
        /* Check fee charged and nonce has advanced */
        let mut recent_message = nonce_tx.message;
        recent_message.recent_blockhash = bank.last_blockhash();
        assert_eq!(
            bank.get_balance(&nonce_pubkey),
            nonce_starting_balance
                - bank
                    .get_fee_for_message(&recent_message.try_into().unwrap())
                    .unwrap()
        );
        assert_ne!(
            nonce_hash,
            get_nonce_blockhash(&bank, &nonce_pubkey).unwrap()
        );
    }

    #[test]
    fn test_nonce_fee_calculator_updates() {
        let (mut genesis_config, mint_keypair) = create_genesis_config(1_000_000);
        genesis_config.rent.lamports_per_byte_year = 0;
        let mut bank = Bank::new_for_tests(&genesis_config);
        bank.feature_set = Arc::new(FeatureSet::all_enabled());
        let mut bank = Arc::new(bank);

        // Deliberately use bank 0 to initialize nonce account, so that nonce account fee_calculator indicates 0 fees
        let (custodian_keypair, nonce_keypair) =
            nonce_setup(&mut bank, &mint_keypair, 500_000, 100_000, None).unwrap();
        let custodian_pubkey = custodian_keypair.pubkey();
        let nonce_pubkey = nonce_keypair.pubkey();

        // Grab the hash and fee_calculator stored in the nonce account
        let (stored_nonce_hash, stored_fee_calculator) = bank
            .get_account(&nonce_pubkey)
            .and_then(|acc| {
                let nonce_versions = StateMut::<nonce::state::Versions>::state(&acc);
                match nonce_versions.ok()?.state() {
                    nonce::State::Initialized(ref data) => {
                        Some((data.blockhash(), data.fee_calculator))
                    }
                    _ => None,
                }
            })
            .unwrap();

        // Kick nonce hash off the blockhash_queue
        for _ in 0..MAX_RECENT_BLOCKHASHES + 1 {
            goto_end_of_slot(Arc::get_mut(&mut bank).unwrap());
            bank = Arc::new(new_from_parent(&bank));
        }

        // Nonce transfer
        let nonce_tx = Transaction::new_signed_with_payer(
            &[
                system_instruction::advance_nonce_account(&nonce_pubkey, &nonce_pubkey),
                system_instruction::transfer(
                    &custodian_pubkey,
                    &solana_sdk::pubkey::new_rand(),
                    100_000,
                ),
            ],
            Some(&custodian_pubkey),
            &[&custodian_keypair, &nonce_keypair],
            stored_nonce_hash,
        );
        bank.process_transaction(&nonce_tx).unwrap();

        // Grab the new hash and fee_calculator; both should be updated
        let (nonce_hash, fee_calculator) = bank
            .get_account(&nonce_pubkey)
            .and_then(|acc| {
                let nonce_versions = StateMut::<nonce::state::Versions>::state(&acc);
                match nonce_versions.ok()?.state() {
                    nonce::State::Initialized(ref data) => {
                        Some((data.blockhash(), data.fee_calculator))
                    }
                    _ => None,
                }
            })
            .unwrap();

        assert_ne!(stored_nonce_hash, nonce_hash);
        assert_ne!(stored_fee_calculator, fee_calculator);
    }

    #[test]
    fn test_nonce_fee_calculator_updates_tx_wide_cap() {
        let (mut genesis_config, mint_keypair) = create_genesis_config(1_000_000);
        genesis_config.rent.lamports_per_byte_year = 0;
        let mut bank = Bank::new_for_tests(&genesis_config);
        bank.feature_set = Arc::new(FeatureSet::all_enabled());
        let mut bank = Arc::new(bank);

        // Deliberately use bank 0 to initialize nonce account, so that nonce account fee_calculator indicates 0 fees
        let (custodian_keypair, nonce_keypair) =
            nonce_setup(&mut bank, &mint_keypair, 500_000, 100_000, None).unwrap();
        let custodian_pubkey = custodian_keypair.pubkey();
        let nonce_pubkey = nonce_keypair.pubkey();

        // Grab the hash and fee_calculator stored in the nonce account
        let (stored_nonce_hash, stored_fee_calculator) = bank
            .get_account(&nonce_pubkey)
            .and_then(|acc| {
                let nonce_versions = StateMut::<nonce::state::Versions>::state(&acc);
                match nonce_versions.ok()?.state() {
                    nonce::State::Initialized(ref data) => {
                        Some((data.blockhash(), data.fee_calculator))
                    }
                    _ => None,
                }
            })
            .unwrap();

        // Kick nonce hash off the blockhash_queue
        for _ in 0..MAX_RECENT_BLOCKHASHES + 1 {
            goto_end_of_slot(Arc::get_mut(&mut bank).unwrap());
            bank = Arc::new(new_from_parent(&bank));
        }

        // Nonce transfer
        let nonce_tx = Transaction::new_signed_with_payer(
            &[
                system_instruction::advance_nonce_account(&nonce_pubkey, &nonce_pubkey),
                system_instruction::transfer(
                    &custodian_pubkey,
                    &solana_sdk::pubkey::new_rand(),
                    100_000,
                ),
            ],
            Some(&custodian_pubkey),
            &[&custodian_keypair, &nonce_keypair],
            stored_nonce_hash,
        );
        bank.process_transaction(&nonce_tx).unwrap();

        // Grab the new hash and fee_calculator; both should be updated
        let (nonce_hash, fee_calculator) = bank
            .get_account(&nonce_pubkey)
            .and_then(|acc| {
                let nonce_versions = StateMut::<nonce::state::Versions>::state(&acc);
                match nonce_versions.ok()?.state() {
                    nonce::State::Initialized(ref data) => {
                        Some((data.blockhash(), data.fee_calculator))
                    }
                    _ => None,
                }
            })
            .unwrap();

        assert_ne!(stored_nonce_hash, nonce_hash);
        assert_ne!(stored_fee_calculator, fee_calculator);
    }

    #[test]
    fn test_check_ro_durable_nonce_fails() {
        let (mut bank, _mint_keypair, custodian_keypair, nonce_keypair) = setup_nonce_with_bank(
            10_000_000,
            |_| {},
            5_000_000,
            250_000,
            None,
            FeatureSet::all_enabled(),
        )
        .unwrap();
        let custodian_pubkey = custodian_keypair.pubkey();
        let nonce_pubkey = nonce_keypair.pubkey();

        let nonce_hash = get_nonce_blockhash(&bank, &nonce_pubkey).unwrap();
        let account_metas = vec![
            AccountMeta::new_readonly(nonce_pubkey, false),
            #[allow(deprecated)]
            AccountMeta::new_readonly(sysvar::recent_blockhashes::id(), false),
            AccountMeta::new_readonly(nonce_pubkey, true),
        ];
        let nonce_instruction = Instruction::new_with_bincode(
            system_program::id(),
            &system_instruction::SystemInstruction::AdvanceNonceAccount,
            account_metas,
        );
        let tx = Transaction::new_signed_with_payer(
            &[nonce_instruction],
            Some(&custodian_pubkey),
            &[&custodian_keypair, &nonce_keypair],
            nonce_hash,
        );
        // SanitizedMessage::get_durable_nonce returns None because nonce
        // account is not writable. Durable nonce and blockhash domains are
        // separate, so the recent_blockhash (== durable nonce) in the
        // transaction is not found in the hash queue.
        assert_eq!(
            bank.process_transaction(&tx),
            Err(TransactionError::BlockhashNotFound),
        );
        // Kick nonce hash off the blockhash_queue
        for _ in 0..MAX_RECENT_BLOCKHASHES + 1 {
            goto_end_of_slot(Arc::get_mut(&mut bank).unwrap());
            bank = Arc::new(new_from_parent(&bank));
        }
        // Caught by the runtime because it is a nonce transaction
        assert_eq!(
            bank.process_transaction(&tx),
            Err(TransactionError::BlockhashNotFound)
        );
        assert_eq!(
            bank.check_transaction_for_nonce(
                &SanitizedTransaction::from_transaction_for_tests(tx),
                &bank.next_durable_nonce(),
            ),
            None
        );
    }

    #[test]
    fn test_collect_balances() {
        let parent = create_simple_test_arc_bank(500);
        let bank0 = Arc::new(new_from_parent(&parent));

        let keypair = Keypair::new();
        let pubkey0 = solana_sdk::pubkey::new_rand();
        let pubkey1 = solana_sdk::pubkey::new_rand();
        let program_id = Pubkey::new(&[2; 32]);
        let keypair_account = AccountSharedData::new(8, 0, &program_id);
        let account0 = AccountSharedData::new(11, 0, &program_id);
        let program_account = AccountSharedData::new(1, 10, &Pubkey::default());
        bank0.store_account(&keypair.pubkey(), &keypair_account);
        bank0.store_account(&pubkey0, &account0);
        bank0.store_account(&program_id, &program_account);

        let instructions = vec![CompiledInstruction::new(1, &(), vec![0])];
        let tx0 = Transaction::new_with_compiled_instructions(
            &[&keypair],
            &[pubkey0],
            Hash::default(),
            vec![program_id],
            instructions,
        );
        let instructions = vec![CompiledInstruction::new(1, &(), vec![0])];
        let tx1 = Transaction::new_with_compiled_instructions(
            &[&keypair],
            &[pubkey1],
            Hash::default(),
            vec![program_id],
            instructions,
        );
        let txs = vec![tx0, tx1];
        let batch = bank0.prepare_batch_for_tests(txs.clone());
        let balances = bank0.collect_balances(&batch);
        assert_eq!(balances.len(), 2);
        assert_eq!(balances[0], vec![8, 11, 1]);
        assert_eq!(balances[1], vec![8, 0, 1]);

        let txs: Vec<_> = txs.into_iter().rev().collect();
        let batch = bank0.prepare_batch_for_tests(txs);
        let balances = bank0.collect_balances(&batch);
        assert_eq!(balances.len(), 2);
        assert_eq!(balances[0], vec![8, 0, 1]);
        assert_eq!(balances[1], vec![8, 11, 1]);
    }

    #[test]
    fn test_pre_post_transaction_balances() {
        let (mut genesis_config, _mint_keypair) = create_genesis_config(500_000);
        let fee_rate_governor = FeeRateGovernor::new(5000, 0);
        genesis_config.fee_rate_governor = fee_rate_governor;
        let parent = Arc::new(Bank::new_for_tests(&genesis_config));
        let bank0 = Arc::new(new_from_parent(&parent));

        let keypair0 = Keypair::new();
        let keypair1 = Keypair::new();
        let pubkey0 = solana_sdk::pubkey::new_rand();
        let pubkey1 = solana_sdk::pubkey::new_rand();
        let pubkey2 = solana_sdk::pubkey::new_rand();
        let keypair0_account = AccountSharedData::new(8_000, 0, &Pubkey::default());
        let keypair1_account = AccountSharedData::new(9_000, 0, &Pubkey::default());
        let account0 = AccountSharedData::new(11_000, 0, &Pubkey::default());
        bank0.store_account(&keypair0.pubkey(), &keypair0_account);
        bank0.store_account(&keypair1.pubkey(), &keypair1_account);
        bank0.store_account(&pubkey0, &account0);

        let blockhash = bank0.last_blockhash();

        let tx0 = system_transaction::transfer(&keypair0, &pubkey0, 2_000, blockhash);
        let tx1 = system_transaction::transfer(&Keypair::new(), &pubkey1, 2_000, blockhash);
        let tx2 = system_transaction::transfer(&keypair1, &pubkey2, 12_000, blockhash);
        let txs = vec![tx0, tx1, tx2];

        let lock_result = bank0.prepare_batch_for_tests(txs);
        let (transaction_results, transaction_balances_set) = bank0
            .load_execute_and_commit_transactions(
                &lock_result,
                MAX_PROCESSING_AGE,
                true,
                false,
                false,
                false,
                &mut ExecuteTimings::default(),
                None,
            );

        assert_eq!(transaction_balances_set.pre_balances.len(), 3);
        assert_eq!(transaction_balances_set.post_balances.len(), 3);

        assert!(transaction_results.execution_results[0].was_executed_successfully());
        assert_eq!(
            transaction_balances_set.pre_balances[0],
            vec![8_000, 11_000, 1]
        );
        assert_eq!(
            transaction_balances_set.post_balances[0],
            vec![1_000, 13_000, 1]
        );

        // Failed transactions still produce balance sets
        // This is a TransactionError - not possible to charge fees
        assert!(matches!(
            transaction_results.execution_results[1],
            TransactionExecutionResult::NotExecuted(TransactionError::AccountNotFound),
        ));
        assert_eq!(transaction_balances_set.pre_balances[1], vec![0, 0, 1]);
        assert_eq!(transaction_balances_set.post_balances[1], vec![0, 0, 1]);

        // Failed transactions still produce balance sets
        // This is an InstructionError - fees charged
        assert!(matches!(
            transaction_results.execution_results[2],
            TransactionExecutionResult::Executed {
                details: TransactionExecutionDetails {
                    status: Err(TransactionError::InstructionError(
                        0,
                        InstructionError::Custom(1),
                    )),
                    ..
                },
                ..
            },
        ));
        assert_eq!(transaction_balances_set.pre_balances[2], vec![9_000, 0, 1]);
        assert_eq!(transaction_balances_set.post_balances[2], vec![4_000, 0, 1]);
    }

    #[test]
    fn test_transaction_with_duplicate_accounts_in_instruction() {
        let (genesis_config, mint_keypair) = create_genesis_config(500);
        let mut bank = Bank::new_for_tests(&genesis_config);

        fn mock_process_instruction(
            _first_instruction_account: IndexOfAccount,
            invoke_context: &mut InvokeContext,
        ) -> result::Result<(), InstructionError> {
            let transaction_context = &invoke_context.transaction_context;
            let instruction_context = transaction_context.get_current_instruction_context()?;
            let instruction_data = instruction_context.get_instruction_data();
            let lamports = u64::from_le_bytes(instruction_data.try_into().unwrap());
            instruction_context
                .try_borrow_instruction_account(transaction_context, 2)?
                .checked_sub_lamports(lamports)?;
            instruction_context
                .try_borrow_instruction_account(transaction_context, 1)?
                .checked_add_lamports(lamports)?;
            instruction_context
                .try_borrow_instruction_account(transaction_context, 0)?
                .checked_sub_lamports(lamports)?;
            instruction_context
                .try_borrow_instruction_account(transaction_context, 1)?
                .checked_add_lamports(lamports)?;
            Ok(())
        }

        let mock_program_id = Pubkey::new(&[2u8; 32]);
        bank.add_builtin("mock_program", &mock_program_id, mock_process_instruction);

        let from_pubkey = solana_sdk::pubkey::new_rand();
        let to_pubkey = solana_sdk::pubkey::new_rand();
        let dup_pubkey = from_pubkey;
        let from_account = AccountSharedData::new(sol_to_lamports(100.), 1, &mock_program_id);
        let to_account = AccountSharedData::new(0, 1, &mock_program_id);
        bank.store_account(&from_pubkey, &from_account);
        bank.store_account(&to_pubkey, &to_account);

        let account_metas = vec![
            AccountMeta::new(from_pubkey, false),
            AccountMeta::new(to_pubkey, false),
            AccountMeta::new(dup_pubkey, false),
        ];
        let instruction =
            Instruction::new_with_bincode(mock_program_id, &sol_to_lamports(10.), account_metas);
        let tx = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&mint_keypair.pubkey()),
            &[&mint_keypair],
            bank.last_blockhash(),
        );

        let result = bank.process_transaction(&tx);
        assert_eq!(result, Ok(()));
        assert_eq!(bank.get_balance(&from_pubkey), sol_to_lamports(80.));
        assert_eq!(bank.get_balance(&to_pubkey), sol_to_lamports(20.));
    }

    #[test]
    fn test_transaction_with_program_ids_passed_to_programs() {
        let (genesis_config, mint_keypair) = create_genesis_config(500);
        let mut bank = Bank::new_for_tests(&genesis_config);

        #[allow(clippy::unnecessary_wraps)]
        fn mock_process_instruction(
            _first_instruction_account: IndexOfAccount,
            _invoke_context: &mut InvokeContext,
        ) -> result::Result<(), InstructionError> {
            Ok(())
        }

        let mock_program_id = Pubkey::new(&[2u8; 32]);
        bank.add_builtin("mock_program", &mock_program_id, mock_process_instruction);

        let from_pubkey = solana_sdk::pubkey::new_rand();
        let to_pubkey = solana_sdk::pubkey::new_rand();
        let dup_pubkey = from_pubkey;
        let from_account = AccountSharedData::new(100, 1, &mock_program_id);
        let to_account = AccountSharedData::new(0, 1, &mock_program_id);
        bank.store_account(&from_pubkey, &from_account);
        bank.store_account(&to_pubkey, &to_account);

        let account_metas = vec![
            AccountMeta::new(from_pubkey, false),
            AccountMeta::new(to_pubkey, false),
            AccountMeta::new(dup_pubkey, false),
            AccountMeta::new(mock_program_id, false),
        ];
        let instruction = Instruction::new_with_bincode(mock_program_id, &10, account_metas);
        let tx = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&mint_keypair.pubkey()),
            &[&mint_keypair],
            bank.last_blockhash(),
        );

        let result = bank.process_transaction(&tx);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn test_account_ids_after_program_ids() {
        solana_logger::setup();
        let (genesis_config, mint_keypair) = create_genesis_config(500);
        let mut bank = Bank::new_for_tests(&genesis_config);

        let from_pubkey = solana_sdk::pubkey::new_rand();
        let to_pubkey = solana_sdk::pubkey::new_rand();

        let account_metas = vec![
            AccountMeta::new(from_pubkey, false),
            AccountMeta::new(to_pubkey, false),
        ];

        let instruction =
            Instruction::new_with_bincode(solana_vote_program::id(), &10, account_metas);
        let mut tx = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&mint_keypair.pubkey()),
            &[&mint_keypair],
            bank.last_blockhash(),
        );

        tx.message.account_keys.push(solana_sdk::pubkey::new_rand());

        bank.add_builtin(
            "mock_vote",
            &solana_vote_program::id(),
            mock_ok_vote_processor,
        );
        let result = bank.process_transaction(&tx);
        assert_eq!(result, Ok(()));
        let account = bank.get_account(&solana_vote_program::id()).unwrap();
        info!("account: {:?}", account);
        assert!(account.executable());
    }

    #[test]
    fn test_incinerator() {
        let (genesis_config, mint_keypair) = create_genesis_config(1_000_000_000_000);
        let bank0 = Arc::new(Bank::new_for_tests(&genesis_config));

        // Move to the first normal slot so normal rent behaviour applies
        let bank = Bank::new_from_parent(
            &bank0,
            &Pubkey::default(),
            genesis_config.epoch_schedule.first_normal_slot,
        );
        let pre_capitalization = bank.capitalization();

        // Burn a non-rent exempt amount
        let burn_amount = bank.get_minimum_balance_for_rent_exemption(0) - 1;

        assert_eq!(bank.get_balance(&incinerator::id()), 0);
        bank.transfer(burn_amount, &mint_keypair, &incinerator::id())
            .unwrap();
        assert_eq!(bank.get_balance(&incinerator::id()), burn_amount);
        bank.freeze();
        assert_eq!(bank.get_balance(&incinerator::id()), 0);

        // Ensure that no rent was collected, and the entire burn amount was removed from bank
        // capitalization
        assert_eq!(bank.capitalization(), pre_capitalization - burn_amount);
    }

    #[test]
    fn test_duplicate_account_key() {
        solana_logger::setup();
        let (genesis_config, mint_keypair) = create_genesis_config(500);
        let mut bank = Bank::new_for_tests(&genesis_config);

        let from_pubkey = solana_sdk::pubkey::new_rand();
        let to_pubkey = solana_sdk::pubkey::new_rand();

        let account_metas = vec![
            AccountMeta::new(from_pubkey, false),
            AccountMeta::new(to_pubkey, false),
        ];

        bank.add_builtin(
            "mock_vote",
            &solana_vote_program::id(),
            mock_ok_vote_processor,
        );

        let instruction =
            Instruction::new_with_bincode(solana_vote_program::id(), &10, account_metas);
        let mut tx = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&mint_keypair.pubkey()),
            &[&mint_keypair],
            bank.last_blockhash(),
        );
        tx.message.account_keys.push(from_pubkey);

        let result = bank.process_transaction(&tx);
        assert_eq!(result, Err(TransactionError::AccountLoadedTwice));
    }

    #[test]
    fn test_process_transaction_with_too_many_account_locks() {
        solana_logger::setup();
        let (genesis_config, mint_keypair) = create_genesis_config(500);
        let mut bank = Bank::new_for_tests(&genesis_config);

        let from_pubkey = solana_sdk::pubkey::new_rand();
        let to_pubkey = solana_sdk::pubkey::new_rand();

        let account_metas = vec![
            AccountMeta::new(from_pubkey, false),
            AccountMeta::new(to_pubkey, false),
        ];

        bank.add_builtin(
            "mock_vote",
            &solana_vote_program::id(),
            mock_ok_vote_processor,
        );

        let instruction =
            Instruction::new_with_bincode(solana_vote_program::id(), &10, account_metas);
        let mut tx = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&mint_keypair.pubkey()),
            &[&mint_keypair],
            bank.last_blockhash(),
        );

        let transaction_account_lock_limit = bank.get_transaction_account_lock_limit();
        while tx.message.account_keys.len() <= transaction_account_lock_limit {
            tx.message.account_keys.push(solana_sdk::pubkey::new_rand());
        }

        let result = bank.process_transaction(&tx);
        assert_eq!(result, Err(TransactionError::TooManyAccountLocks));
    }

    #[test]
    fn test_program_id_as_payer() {
        solana_logger::setup();
        let (genesis_config, mint_keypair) = create_genesis_config(500);
        let mut bank = Bank::new_for_tests(&genesis_config);

        let from_pubkey = solana_sdk::pubkey::new_rand();
        let to_pubkey = solana_sdk::pubkey::new_rand();

        let account_metas = vec![
            AccountMeta::new(from_pubkey, false),
            AccountMeta::new(to_pubkey, false),
        ];

        bank.add_builtin(
            "mock_vote",
            &solana_vote_program::id(),
            mock_ok_vote_processor,
        );

        let instruction =
            Instruction::new_with_bincode(solana_vote_program::id(), &10, account_metas);
        let mut tx = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&mint_keypair.pubkey()),
            &[&mint_keypair],
            bank.last_blockhash(),
        );

        info!(
            "mint: {} account keys: {:?}",
            mint_keypair.pubkey(),
            tx.message.account_keys
        );
        assert_eq!(tx.message.account_keys.len(), 4);
        tx.message.account_keys.clear();
        tx.message.account_keys.push(solana_vote_program::id());
        tx.message.account_keys.push(mint_keypair.pubkey());
        tx.message.account_keys.push(from_pubkey);
        tx.message.account_keys.push(to_pubkey);
        tx.message.instructions[0].program_id_index = 0;
        tx.message.instructions[0].accounts.clear();
        tx.message.instructions[0].accounts.push(2);
        tx.message.instructions[0].accounts.push(3);

        let result = bank.process_transaction(&tx);
        assert_eq!(result, Err(TransactionError::SanitizeFailure));
    }

    #[allow(clippy::unnecessary_wraps)]
    fn mock_ok_vote_processor(
        _first_instruction_account: IndexOfAccount,
        _invoke_context: &mut InvokeContext,
    ) -> std::result::Result<(), InstructionError> {
        Ok(())
    }

    #[test]
    fn test_ref_account_key_after_program_id() {
        let (genesis_config, mint_keypair) = create_genesis_config(500);
        let mut bank = Bank::new_for_tests(&genesis_config);

        let from_pubkey = solana_sdk::pubkey::new_rand();
        let to_pubkey = solana_sdk::pubkey::new_rand();

        let account_metas = vec![
            AccountMeta::new(from_pubkey, false),
            AccountMeta::new(to_pubkey, false),
        ];

        bank.add_builtin(
            "mock_vote",
            &solana_vote_program::id(),
            mock_ok_vote_processor,
        );

        let instruction =
            Instruction::new_with_bincode(solana_vote_program::id(), &10, account_metas);
        let mut tx = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&mint_keypair.pubkey()),
            &[&mint_keypair],
            bank.last_blockhash(),
        );

        tx.message.account_keys.push(solana_sdk::pubkey::new_rand());
        assert_eq!(tx.message.account_keys.len(), 5);
        tx.message.instructions[0].accounts.remove(0);
        tx.message.instructions[0].accounts.push(4);

        let result = bank.process_transaction(&tx);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn test_fuzz_instructions() {
        solana_logger::setup();
        use rand::{thread_rng, Rng};
        let mut bank = create_simple_test_bank(1_000_000_000);

        let max_programs = 5;
        let program_keys: Vec<_> = (0..max_programs)
            .enumerate()
            .map(|i| {
                let key = solana_sdk::pubkey::new_rand();
                let name = format!("program{:?}", i);
                bank.add_builtin(&name, &key, mock_ok_vote_processor);
                (key, name.as_bytes().to_vec())
            })
            .collect();
        let max_keys = 100;
        let keys: Vec<_> = (0..max_keys)
            .enumerate()
            .map(|_| {
                let key = solana_sdk::pubkey::new_rand();
                let balance = if thread_rng().gen_ratio(9, 10) {
                    let lamports = if thread_rng().gen_ratio(1, 5) {
                        thread_rng().gen_range(0, 10)
                    } else {
                        thread_rng().gen_range(20, 100)
                    };
                    let space = thread_rng().gen_range(0, 10);
                    let owner = Pubkey::default();
                    let account = AccountSharedData::new(lamports, space, &owner);
                    bank.store_account(&key, &account);
                    lamports
                } else {
                    0
                };
                (key, balance)
            })
            .collect();
        let mut results = HashMap::new();
        for _ in 0..2_000 {
            let num_keys = if thread_rng().gen_ratio(1, 5) {
                thread_rng().gen_range(0, max_keys)
            } else {
                thread_rng().gen_range(1, 4)
            };
            let num_instructions = thread_rng().gen_range(0, max_keys - num_keys);

            let mut account_keys: Vec<_> = if thread_rng().gen_ratio(1, 5) {
                (0..num_keys)
                    .map(|_| {
                        let idx = thread_rng().gen_range(0, keys.len());
                        keys[idx].0
                    })
                    .collect()
            } else {
                let mut inserted = HashSet::new();
                (0..num_keys)
                    .map(|_| {
                        let mut idx;
                        loop {
                            idx = thread_rng().gen_range(0, keys.len());
                            if !inserted.contains(&idx) {
                                break;
                            }
                        }
                        inserted.insert(idx);
                        keys[idx].0
                    })
                    .collect()
            };

            let instructions: Vec<_> = if num_keys > 0 {
                (0..num_instructions)
                    .map(|_| {
                        let num_accounts_to_pass = thread_rng().gen_range(0, num_keys);
                        let account_indexes = (0..num_accounts_to_pass)
                            .map(|_| thread_rng().gen_range(0, num_keys))
                            .collect();
                        let program_index: u8 = thread_rng().gen_range(0, num_keys) as u8;
                        if thread_rng().gen_ratio(4, 5) {
                            let programs_index = thread_rng().gen_range(0, program_keys.len());
                            account_keys[program_index as usize] = program_keys[programs_index].0;
                        }
                        CompiledInstruction::new(program_index, &10, account_indexes)
                    })
                    .collect()
            } else {
                vec![]
            };

            let account_keys_len = std::cmp::max(account_keys.len(), 2);
            let num_signatures = if thread_rng().gen_ratio(1, 5) {
                thread_rng().gen_range(0, account_keys_len + 10)
            } else {
                thread_rng().gen_range(1, account_keys_len)
            };

            let num_required_signatures = if thread_rng().gen_ratio(1, 5) {
                thread_rng().gen_range(0, account_keys_len + 10) as u8
            } else {
                thread_rng().gen_range(1, std::cmp::max(2, num_signatures)) as u8
            };
            let num_readonly_signed_accounts = if thread_rng().gen_ratio(1, 5) {
                thread_rng().gen_range(0, account_keys_len) as u8
            } else {
                let max = if num_required_signatures > 1 {
                    num_required_signatures - 1
                } else {
                    1
                };
                thread_rng().gen_range(0, max) as u8
            };

            let num_readonly_unsigned_accounts = if thread_rng().gen_ratio(1, 5)
                || (num_required_signatures as usize) >= account_keys_len
            {
                thread_rng().gen_range(0, account_keys_len) as u8
            } else {
                thread_rng().gen_range(0, account_keys_len - num_required_signatures as usize) as u8
            };

            let header = MessageHeader {
                num_required_signatures,
                num_readonly_signed_accounts,
                num_readonly_unsigned_accounts,
            };
            let message = Message {
                header,
                account_keys,
                recent_blockhash: bank.last_blockhash(),
                instructions,
            };

            let tx = Transaction {
                signatures: vec![Signature::default(); num_signatures],
                message,
            };

            let result = bank.process_transaction(&tx);
            for (key, balance) in &keys {
                assert_eq!(bank.get_balance(key), *balance);
            }
            for (key, name) in &program_keys {
                let account = bank.get_account(key).unwrap();
                assert!(account.executable());
                assert_eq!(account.data(), name);
            }
            info!("result: {:?}", result);
            let result_key = format!("{:?}", result);
            *results.entry(result_key).or_insert(0) += 1;
        }
        info!("results: {:?}", results);
    }

    #[test]
    fn test_bank_hash_consistency() {
        solana_logger::setup();

        let mut genesis_config = GenesisConfig::new(
            &[(
                Pubkey::new(&[42; 32]),
                AccountSharedData::new(1_000_000_000_000, 0, &system_program::id()),
            )],
            &[],
        );
        genesis_config.creation_time = 0;
        genesis_config.cluster_type = ClusterType::MainnetBeta;
        genesis_config.rent.burn_percent = 100;
        let mut bank = Arc::new(Bank::new_for_tests(&genesis_config));
        // Check a few slots, cross an epoch boundary
        assert_eq!(bank.get_slots_in_epoch(0), 32);
        loop {
            goto_end_of_slot(Arc::get_mut(&mut bank).unwrap());
            if bank.slot == 0 {
                assert_eq!(
                    bank.hash().to_string(),
                    "5gY6TCgB9NymbbxgFgAjvYLpXjyXiVyyruS1aEwbWKLK"
                );
            }
            if bank.slot == 32 {
                assert_eq!(
                    bank.hash().to_string(),
                    "6uJ5C4QDXWCN39EjJ5Frcz73nnS2jMJ55KgkQff12Fqp"
                );
            }
            if bank.slot == 64 {
                assert_eq!(
                    bank.hash().to_string(),
                    "4u8bxZRLYdQBkWRBwmpcwcQVMCJoEpzY7hCuAzxr3kCe"
                );
            }
            if bank.slot == 128 {
                assert_eq!(
                    bank.hash().to_string(),
                    "4c5F8UbcDD8FM7qXcfv6BPPo6nHNYJQmN5gHiCMTdEzX"
                );
                break;
            }
            bank = Arc::new(new_from_parent(&bank));
        }
    }

    #[test]
    fn test_same_program_id_uses_unqiue_executable_accounts() {
        fn nested_processor(
            _first_instruction_account: IndexOfAccount,
            invoke_context: &mut InvokeContext,
        ) -> result::Result<(), InstructionError> {
            let transaction_context = &invoke_context.transaction_context;
            let instruction_context = transaction_context.get_current_instruction_context()?;
            let _ = instruction_context
                .try_borrow_program_account(transaction_context, 1)?
                .checked_add_lamports(1);
            Ok(())
        }

        let (genesis_config, mint_keypair) = create_genesis_config(50000);
        let mut bank = Bank::new_for_tests(&genesis_config);

        // Add a new program
        let program1_pubkey = solana_sdk::pubkey::new_rand();
        bank.add_builtin("program", &program1_pubkey, nested_processor);

        // Add a new program owned by the first
        let program2_pubkey = solana_sdk::pubkey::new_rand();
        let mut program2_account = AccountSharedData::new(42, 1, &program1_pubkey);
        program2_account.set_executable(true);
        bank.store_account(&program2_pubkey, &program2_account);

        let instruction = Instruction::new_with_bincode(program2_pubkey, &10, vec![]);
        let tx = Transaction::new_signed_with_payer(
            &[instruction.clone(), instruction],
            Some(&mint_keypair.pubkey()),
            &[&mint_keypair],
            bank.last_blockhash(),
        );
        assert!(bank.process_transaction(&tx).is_ok());
        assert_eq!(1, bank.get_balance(&program1_pubkey));
        assert_eq!(42, bank.get_balance(&program2_pubkey));
    }

    fn get_shrink_account_size() -> usize {
        let (genesis_config, _mint_keypair) = create_genesis_config(1_000_000_000);

        // Set root for bank 0, with caching disabled so we can get the size
        // of the storage for this slot
        let mut bank0 = Arc::new(Bank::new_with_config_for_tests(
            &genesis_config,
            AccountSecondaryIndexes::default(),
            false,
            AccountShrinkThreshold::default(),
        ));
        bank0.restore_old_behavior_for_fragile_tests();
        goto_end_of_slot(Arc::<Bank>::get_mut(&mut bank0).unwrap());
        bank0.freeze();
        bank0.squash();

        let sizes = bank0
            .rc
            .accounts
            .scan_slot(0, |stored_account| Some(stored_account.stored_size()));

        // Create an account such that it takes DEFAULT_ACCOUNTS_SHRINK_RATIO of the total account space for
        // the slot, so when it gets pruned, the storage entry will become a shrink candidate.
        let bank0_total_size: usize = sizes.into_iter().sum();
        let pubkey0_size = (bank0_total_size as f64 / (1.0 - DEFAULT_ACCOUNTS_SHRINK_RATIO)).ceil();
        assert!(
            pubkey0_size / (pubkey0_size + bank0_total_size as f64) > DEFAULT_ACCOUNTS_SHRINK_RATIO
        );
        pubkey0_size as usize
    }

    #[test]
    fn test_clean_nonrooted() {
        solana_logger::setup();

        let (genesis_config, _mint_keypair) = create_genesis_config(1_000_000_000);
        let pubkey0 = Pubkey::new(&[0; 32]);
        let pubkey1 = Pubkey::new(&[1; 32]);

        info!("pubkey0: {}", pubkey0);
        info!("pubkey1: {}", pubkey1);

        // Set root for bank 0, with caching enabled
        let mut bank0 = Arc::new(Bank::new_with_config_for_tests(
            &genesis_config,
            AccountSecondaryIndexes::default(),
            true,
            AccountShrinkThreshold::default(),
        ));

        let account_zero = AccountSharedData::new(0, 0, &Pubkey::new_unique());

        goto_end_of_slot(Arc::<Bank>::get_mut(&mut bank0).unwrap());
        bank0.freeze();
        bank0.squash();
        // Flush now so that accounts cache cleaning doesn't clean up bank 0 when later
        // slots add updates to the cache
        bank0.force_flush_accounts_cache();

        // Store some lamports in bank 1
        let some_lamports = 123;
        let mut bank1 = Arc::new(Bank::new_from_parent(&bank0, &Pubkey::default(), 1));
        bank1.deposit(&pubkey0, some_lamports).unwrap();
        goto_end_of_slot(Arc::<Bank>::get_mut(&mut bank1).unwrap());
        bank1.freeze();
        bank1.flush_accounts_cache_slot_for_tests();

        bank1.print_accounts_stats();

        // Store some lamports for pubkey1 in bank 2, root bank 2
        // bank2's parent is bank0
        let mut bank2 = Arc::new(Bank::new_from_parent(&bank0, &Pubkey::default(), 2));
        bank2.deposit(&pubkey1, some_lamports).unwrap();
        bank2.store_account(&pubkey0, &account_zero);
        goto_end_of_slot(Arc::<Bank>::get_mut(&mut bank2).unwrap());
        bank2.freeze();
        bank2.squash();
        bank2.force_flush_accounts_cache();

        bank2.print_accounts_stats();
        drop(bank1);

        // Clean accounts, which should add earlier slots to the shrink
        // candidate set
        bank2.clean_accounts_for_tests();

        let mut bank3 = Arc::new(Bank::new_from_parent(&bank2, &Pubkey::default(), 3));
        bank3.deposit(&pubkey1, some_lamports + 1).unwrap();
        goto_end_of_slot(Arc::<Bank>::get_mut(&mut bank3).unwrap());
        bank3.freeze();
        bank3.squash();
        bank3.force_flush_accounts_cache();

        bank3.clean_accounts_for_tests();
        assert_eq!(
            bank3.rc.accounts.accounts_db.ref_count_for_pubkey(&pubkey0),
            2
        );
        assert!(bank3
            .rc
            .accounts
            .accounts_db
            .storage
            .get_slot_stores(1)
            .is_none());

        bank3.print_accounts_stats();
    }

    #[test]
    fn test_shrink_candidate_slots_cached() {
        solana_logger::setup();

        let (genesis_config, _mint_keypair) = create_genesis_config(1_000_000_000);
        let pubkey0 = solana_sdk::pubkey::new_rand();
        let pubkey1 = solana_sdk::pubkey::new_rand();
        let pubkey2 = solana_sdk::pubkey::new_rand();

        // Set root for bank 0, with caching enabled
        let mut bank0 = Arc::new(Bank::new_with_config_for_tests(
            &genesis_config,
            AccountSecondaryIndexes::default(),
            true,
            AccountShrinkThreshold::default(),
        ));
        bank0.restore_old_behavior_for_fragile_tests();

        let pubkey0_size = get_shrink_account_size();

        let account0 = AccountSharedData::new(1000, pubkey0_size as usize, &Pubkey::new_unique());
        bank0.store_account(&pubkey0, &account0);

        goto_end_of_slot(Arc::<Bank>::get_mut(&mut bank0).unwrap());
        bank0.freeze();
        bank0.squash();
        // Flush now so that accounts cache cleaning doesn't clean up bank 0 when later
        // slots add updates to the cache
        bank0.force_flush_accounts_cache();

        // Store some lamports in bank 1
        let some_lamports = 123;
        let mut bank1 = Arc::new(new_from_parent(&bank0));
        bank1.deposit(&pubkey1, some_lamports).unwrap();
        bank1.deposit(&pubkey2, some_lamports).unwrap();
        goto_end_of_slot(Arc::<Bank>::get_mut(&mut bank1).unwrap());
        bank1.freeze();
        bank1.squash();
        // Flush now so that accounts cache cleaning doesn't clean up bank 0 when later
        // slots add updates to the cache
        bank1.force_flush_accounts_cache();

        // Store some lamports for pubkey1 in bank 2, root bank 2
        let mut bank2 = Arc::new(new_from_parent(&bank1));
        bank2.deposit(&pubkey1, some_lamports).unwrap();
        bank2.store_account(&pubkey0, &account0);
        goto_end_of_slot(Arc::<Bank>::get_mut(&mut bank2).unwrap());
        bank2.freeze();
        bank2.squash();
        bank2.force_flush_accounts_cache();

        // Clean accounts, which should add earlier slots to the shrink
        // candidate set
        bank2.clean_accounts_for_tests();

        // Slots 0 and 1 should be candidates for shrinking, but slot 2
        // shouldn't because none of its accounts are outdated by a later
        // root
        assert_eq!(bank2.shrink_candidate_slots(), 2);
        let alive_counts: Vec<usize> = (0..3)
            .map(|slot| {
                bank2
                    .rc
                    .accounts
                    .accounts_db
                    .alive_account_count_in_slot(slot)
            })
            .collect();

        // No more slots should be shrunk
        assert_eq!(bank2.shrink_candidate_slots(), 0);
        // alive_counts represents the count of alive accounts in the three slots 0,1,2
        assert_eq!(alive_counts, vec![11, 1, 7]);
    }

    #[test]
    fn test_process_stale_slot_with_budget() {
        solana_logger::setup();
        let pubkey1 = solana_sdk::pubkey::new_rand();
        let pubkey2 = solana_sdk::pubkey::new_rand();

        let mut bank = create_simple_test_arc_bank(1_000_000_000);
        bank.restore_old_behavior_for_fragile_tests();
        assert_eq!(bank.process_stale_slot_with_budget(0, 0), 0);
        assert_eq!(bank.process_stale_slot_with_budget(133, 0), 133);

        assert_eq!(bank.process_stale_slot_with_budget(0, 100), 0);
        assert_eq!(bank.process_stale_slot_with_budget(33, 100), 0);
        assert_eq!(bank.process_stale_slot_with_budget(133, 100), 33);

        goto_end_of_slot(Arc::<Bank>::get_mut(&mut bank).unwrap());

        bank.squash();

        let some_lamports = 123;
        let mut bank = Arc::new(new_from_parent(&bank));
        bank.deposit(&pubkey1, some_lamports).unwrap();
        bank.deposit(&pubkey2, some_lamports).unwrap();

        goto_end_of_slot(Arc::<Bank>::get_mut(&mut bank).unwrap());

        let mut bank = Arc::new(new_from_parent(&bank));
        bank.deposit(&pubkey1, some_lamports).unwrap();

        goto_end_of_slot(Arc::<Bank>::get_mut(&mut bank).unwrap());

        bank.squash();
        bank.clean_accounts_for_tests();
        let force_to_return_alive_account = 0;
        assert_eq!(
            bank.process_stale_slot_with_budget(22, force_to_return_alive_account),
            22
        );

        let consumed_budgets: usize = (0..3)
            .map(|_| bank.process_stale_slot_with_budget(0, force_to_return_alive_account))
            .sum();
        // consumed_budgets represents the count of alive accounts in the three slots 0,1,2
        assert_eq!(consumed_budgets, 12);
    }

    #[test]
    fn test_add_builtin_no_overwrite() {
        #[allow(clippy::unnecessary_wraps)]
        fn mock_ix_processor(
            _first_instruction_account: IndexOfAccount,
            _invoke_context: &mut InvokeContext,
        ) -> std::result::Result<(), InstructionError> {
            Ok(())
        }

        let slot = 123;
        let program_id = solana_sdk::pubkey::new_rand();

        let mut bank = Arc::new(Bank::new_from_parent(
            &create_simple_test_arc_bank(100_000),
            &Pubkey::default(),
            slot,
        ));
        assert_eq!(bank.get_account_modified_slot(&program_id), None);

        Arc::get_mut(&mut bank).unwrap().add_builtin(
            "mock_program",
            &program_id,
            mock_ix_processor,
        );
        assert_eq!(bank.get_account_modified_slot(&program_id).unwrap().1, slot);

        let mut bank = Arc::new(new_from_parent(&bank));
        Arc::get_mut(&mut bank).unwrap().add_builtin(
            "mock_program",
            &program_id,
            mock_ix_processor,
        );
        assert_eq!(bank.get_account_modified_slot(&program_id).unwrap().1, slot);
    }

    #[test]
    fn test_add_builtin_loader_no_overwrite() {
        #[allow(clippy::unnecessary_wraps)]
        fn mock_ix_processor(
            _first_instruction_account: IndexOfAccount,
            _context: &mut InvokeContext,
        ) -> std::result::Result<(), InstructionError> {
            Ok(())
        }

        let slot = 123;
        let loader_id = solana_sdk::pubkey::new_rand();

        let mut bank = Arc::new(Bank::new_from_parent(
            &create_simple_test_arc_bank(100_000),
            &Pubkey::default(),
            slot,
        ));
        assert_eq!(bank.get_account_modified_slot(&loader_id), None);

        Arc::get_mut(&mut bank)
            .unwrap()
            .add_builtin("mock_program", &loader_id, mock_ix_processor);
        assert_eq!(bank.get_account_modified_slot(&loader_id).unwrap().1, slot);

        let mut bank = Arc::new(new_from_parent(&bank));
        Arc::get_mut(&mut bank)
            .unwrap()
            .add_builtin("mock_program", &loader_id, mock_ix_processor);
        assert_eq!(bank.get_account_modified_slot(&loader_id).unwrap().1, slot);
    }

    #[test]
    fn test_add_builtin_account() {
        let (mut genesis_config, _mint_keypair) = create_genesis_config(100_000);
        activate_all_features(&mut genesis_config);

        let slot = 123;
        let program_id = solana_sdk::pubkey::new_rand();

        let bank = Arc::new(Bank::new_from_parent(
            &Arc::new(Bank::new_for_tests(&genesis_config)),
            &Pubkey::default(),
            slot,
        ));
        assert_eq!(bank.get_account_modified_slot(&program_id), None);

        assert_capitalization_diff(
            &bank,
            || bank.add_builtin_account("mock_program", &program_id, false),
            |old, new| {
                assert_eq!(old + 1, new);
            },
        );

        assert_eq!(bank.get_account_modified_slot(&program_id).unwrap().1, slot);

        let bank = Arc::new(new_from_parent(&bank));
        assert_capitalization_diff(
            &bank,
            || bank.add_builtin_account("mock_program", &program_id, false),
            |old, new| assert_eq!(old, new),
        );

        assert_eq!(bank.get_account_modified_slot(&program_id).unwrap().1, slot);

        let bank = Arc::new(new_from_parent(&bank));
        // When replacing builtin_program, name must change to disambiguate from repeated
        // invocations.
        assert_capitalization_diff(
            &bank,
            || bank.add_builtin_account("mock_program v2", &program_id, true),
            |old, new| assert_eq!(old, new),
        );

        assert_eq!(
            bank.get_account_modified_slot(&program_id).unwrap().1,
            bank.slot()
        );

        let bank = Arc::new(new_from_parent(&bank));
        assert_capitalization_diff(
            &bank,
            || bank.add_builtin_account("mock_program v2", &program_id, true),
            |old, new| assert_eq!(old, new),
        );

        // replacing with same name shouldn't update account
        assert_eq!(
            bank.get_account_modified_slot(&program_id).unwrap().1,
            bank.parent_slot()
        );
    }

    #[test]
    fn test_add_builtin_account_inherited_cap_while_replacing() {
        let (genesis_config, mint_keypair) = create_genesis_config(100_000);
        let bank = Bank::new_for_tests(&genesis_config);
        let program_id = solana_sdk::pubkey::new_rand();

        bank.add_builtin_account("mock_program", &program_id, false);
        assert_eq!(bank.capitalization(), bank.calculate_capitalization(true));

        // someone mess with program_id's balance
        bank.withdraw(&mint_keypair.pubkey(), 10).unwrap();
        assert_ne!(bank.capitalization(), bank.calculate_capitalization(true));
        bank.deposit(&program_id, 10).unwrap();
        assert_eq!(bank.capitalization(), bank.calculate_capitalization(true));

        bank.add_builtin_account("mock_program v2", &program_id, true);
        assert_eq!(bank.capitalization(), bank.calculate_capitalization(true));
    }

    #[test]
    fn test_add_builtin_account_squatted_while_not_replacing() {
        let (genesis_config, mint_keypair) = create_genesis_config(100_000);
        let bank = Bank::new_for_tests(&genesis_config);
        let program_id = solana_sdk::pubkey::new_rand();

        // someone managed to squat at program_id!
        bank.withdraw(&mint_keypair.pubkey(), 10).unwrap();
        assert_ne!(bank.capitalization(), bank.calculate_capitalization(true));
        bank.deposit(&program_id, 10).unwrap();
        assert_eq!(bank.capitalization(), bank.calculate_capitalization(true));

        bank.add_builtin_account("mock_program", &program_id, false);
        assert_eq!(bank.capitalization(), bank.calculate_capitalization(true));
    }

    #[test]
    #[should_panic(
        expected = "Can't change frozen bank by adding not-existing new builtin \
                   program (mock_program, CiXgo2KHKSDmDnV1F6B69eWFgNAPiSBjjYvfB4cvRNre). \
                   Maybe, inconsistent program activation is detected on snapshot restore?"
    )]
    fn test_add_builtin_account_after_frozen() {
        let slot = 123;
        let program_id = Pubkey::from_str("CiXgo2KHKSDmDnV1F6B69eWFgNAPiSBjjYvfB4cvRNre").unwrap();

        let bank = Bank::new_from_parent(
            &create_simple_test_arc_bank(100_000),
            &Pubkey::default(),
            slot,
        );
        bank.freeze();

        bank.add_builtin_account("mock_program", &program_id, false);
    }
