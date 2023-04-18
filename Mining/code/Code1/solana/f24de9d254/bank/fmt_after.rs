    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                RewardType::Fee => "fee",
                RewardType::Rent => "rent",
                RewardType::Staking => "staking",
                RewardType::Voting => "voting",
            }
        )
    }
}

pub trait DropCallback: fmt::Debug {
    fn callback(&self, b: &Bank);
    fn clone_box(&self) -> Box<dyn DropCallback + Send + Sync>;
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, AbiExample, Clone, Copy)]
pub struct RewardInfo {
    pub reward_type: RewardType,
    pub lamports: i64,          // Reward amount
    pub post_balance: u64,      // Account balance in lamports after `lamports` was applied
    pub commission: Option<u8>, // Vote account commission when the reward was credited, only present for voting and staking rewards
}

#[derive(Debug, Default)]
pub struct OptionalDropCallback(Option<Box<dyn DropCallback + Send + Sync>>);

#[cfg(RUSTC_WITH_SPECIALIZATION)]
impl AbiExample for OptionalDropCallback {
    fn example() -> Self {
        Self(None)
    }
}

#[derive(Debug, Clone, Default)]
pub struct BuiltinPrograms {
    pub vec: Vec<BuiltinProgram>,
}

#[cfg(RUSTC_WITH_SPECIALIZATION)]
impl AbiExample for BuiltinPrograms {
    fn example() -> Self {
        Self::default()
    }
}

/// Manager for the state of all accounts and programs after processing its entries.
/// AbiExample is needed even without Serialize/Deserialize; actual (de-)serialization
/// are implemented elsewhere for versioning
#[derive(AbiExample, Debug)]
pub struct Bank {
    /// References to accounts, parent and signature status
    pub rc: BankRc,

    /// A cache of signature statuses
    pub status_cache: Arc<RwLock<BankStatusCache>>,

    /// FIFO queue of `recent_blockhash` items
    blockhash_queue: RwLock<BlockhashQueue>,

    /// The set of parents including this bank
    pub ancestors: Ancestors,

    /// Hash of this Bank's state. Only meaningful after freezing.
    hash: RwLock<Hash>,

    /// Hash of this Bank's parent's state
    parent_hash: Hash,

    /// parent's slot
    parent_slot: Slot,

    /// slots to hard fork at
    hard_forks: Arc<RwLock<HardForks>>,

    /// The number of transactions processed without error
    transaction_count: AtomicU64,

    /// The number of transaction errors in this slot
    transaction_error_count: AtomicU64,

    /// The number of transaction entries in this slot
    transaction_entries_count: AtomicU64,

    /// The max number of transaction in an entry in this slot
    transactions_per_entry_max: AtomicU64,

    /// Bank tick height
    tick_height: AtomicU64,

    /// The number of signatures from valid transactions in this slot
    signature_count: AtomicU64,

    /// Total capitalization, used to calculate inflation
    capitalization: AtomicU64,

    // Bank max_tick_height
    max_tick_height: u64,

    /// The number of hashes in each tick. None value means hashing is disabled.
    hashes_per_tick: Option<u64>,

    /// The number of ticks in each slot.
    ticks_per_slot: u64,

    /// length of a slot in ns
    pub ns_per_slot: u128,

    /// genesis time, used for computed clock
    genesis_creation_time: UnixTimestamp,

    /// The number of slots per year, used for inflation
    slots_per_year: f64,

    /// Bank slot (i.e. block)
    slot: Slot,

    bank_id: BankId,

    /// Bank epoch
    epoch: Epoch,

    /// Bank block_height
    block_height: u64,

    /// The pubkey to send transactions fees to.
    collector_id: Pubkey,

    /// Fees that have been collected
    collector_fees: AtomicU64,

    /// Deprecated, do not use
    /// Latest transaction fees for transactions processed by this bank
    pub(crate) fee_calculator: FeeCalculator,

    /// Track cluster signature throughput and adjust fee rate
    pub(crate) fee_rate_governor: FeeRateGovernor,

    /// Rent that has been collected
    collected_rent: AtomicU64,

    /// latest rent collector, knows the epoch
    rent_collector: RentCollector,

    /// initialized from genesis
    epoch_schedule: EpochSchedule,

    /// inflation specs
    inflation: Arc<RwLock<Inflation>>,

    /// cache of vote_account and stake_account state for this fork
    stakes_cache: StakesCache,

    /// staked nodes on epoch boundaries, saved off when a bank.slot() is at
    ///   a leader schedule calculation boundary
    epoch_stakes: HashMap<Epoch, EpochStakes>,

    /// A boolean reflecting whether any entries were recorded into the PoH
    /// stream for the slot == self.slot
    is_delta: AtomicBool,

    /// The builtin programs
    builtin_programs: BuiltinPrograms,

    compute_budget: Option<ComputeBudget>,

    /// Dynamic feature transitions for builtin programs
    #[allow(clippy::rc_buffer)]
    builtin_feature_transitions: Arc<Vec<BuiltinFeatureTransition>>,

    /// Protocol-level rewards that were distributed by this bank
    pub rewards: RwLock<Vec<(Pubkey, RewardInfo)>>,

    pub cluster_type: Option<ClusterType>,

    pub lazy_rent_collection: AtomicBool,

    // this is temporary field only to remove rewards_pool entirely
    pub rewards_pool_pubkeys: Arc<HashSet<Pubkey>>,

    /// Cached executors
    cached_executors: RwLock<CachedExecutors>,

    transaction_debug_keys: Option<Arc<HashSet<Pubkey>>>,

    // Global configuration for how transaction logs should be collected across all banks
    pub transaction_log_collector_config: Arc<RwLock<TransactionLogCollectorConfig>>,

    // Logs from transactions that this Bank executed collected according to the criteria in
    // `transaction_log_collector_config`
    pub transaction_log_collector: Arc<RwLock<TransactionLogCollector>>,

    pub feature_set: Arc<FeatureSet>,

    /// callback function only to be called when dropping and should only be called once
    pub drop_callback: RwLock<OptionalDropCallback>,

    pub freeze_started: AtomicBool,

    vote_only_bank: bool,

    cost_tracker: RwLock<CostTracker>,

    sysvar_cache: RwLock<SysvarCache>,

    /// (Pubkey, account Hash) for each account that would have been rewritten in rent collection for this slot
    pub rewrites_skipped_this_slot: Rewrites,

    /// The initial accounts data size at the start of this Bank, before processing any transactions/etc
    accounts_data_size_initial: u64,
    /// The change to accounts data size in this Bank, due on-chain events (i.e. transactions)
    accounts_data_size_delta_on_chain: AtomicI64,
    /// The change to accounts data size in this Bank, due to off-chain events (i.e. rent collection)
    accounts_data_size_delta_off_chain: AtomicI64,

    /// Transaction fee structure
    pub fee_structure: FeeStructure,
}

struct VoteWithStakeDelegations {
    vote_state: Arc<VoteState>,
    vote_account: AccountSharedData,
    // TODO: use StakeAccount<Delegation> once the old code is deleted.
    delegations: Vec<(Pubkey, StakeAccount<()>)>,
}

struct LoadVoteAndStakeAccountsResult {
    vote_with_stake_delegations_map: DashMap<Pubkey, VoteWithStakeDelegations>,
    invalid_stake_keys: DashMap<Pubkey, InvalidCacheEntryReason>,
    invalid_vote_keys: DashMap<Pubkey, InvalidCacheEntryReason>,
    invalid_cached_vote_accounts: usize,
    invalid_cached_stake_accounts: usize,
    invalid_cached_stake_accounts_rent_epoch: usize,
    vote_accounts_cache_miss_count: usize,
}

#[derive(Debug, Default)]
pub struct NewBankOptions {
    pub vote_only_bank: bool,
}

#[derive(Debug)]
struct PrevEpochInflationRewards {
    validator_rewards: u64,
    prev_epoch_duration_in_years: f64,
    validator_rate: f64,
    foundation_rate: f64,
}

pub struct CommitTransactionCounts {
    pub committed_transactions_count: u64,
    pub committed_with_failure_result_count: u64,
    pub signature_count: u64,
}

struct StakeReward {
    stake_pubkey: Pubkey,
    stake_reward_info: RewardInfo,
    stake_account: AccountSharedData,
}

impl StakeReward {
    pub fn get_stake_reward(&self) -> i64 {
        self.stake_reward_info.lamports
    }
}

/// allow [StakeReward] to be passed to `StoreAccounts` directly without copies or vec construction
impl<'a> StorableAccounts<'a, AccountSharedData> for (Slot, &'a [StakeReward]) {
    fn pubkey(&self, index: usize) -> &Pubkey {
        &self.1[index].stake_pubkey
    }
    fn account(&self, index: usize) -> &AccountSharedData {
        &self.1[index].stake_account
    }
    fn slot(&self, _index: usize) -> Slot {
        // per-index slot is not unique per slot when per-account slot is not included in the source data
        self.target_slot()
    }
    fn target_slot(&self) -> Slot {
        self.0
    }
    fn len(&self) -> usize {
        self.1.len()
    }
    fn contains_multiple_slots(&self) -> bool {
        false
    }
}

impl Bank {
    pub fn default_for_tests() -> Self {
        Self::default_with_accounts(Accounts::default_for_tests())
    }

    pub fn new_for_benches(genesis_config: &GenesisConfig) -> Self {
        Self::new_with_paths_for_benches(
            genesis_config,
            Vec::new(),
            None,
            None,
            AccountSecondaryIndexes::default(),
            false,
            AccountShrinkThreshold::default(),
            false,
        )
    }

    pub fn new_for_tests(genesis_config: &GenesisConfig) -> Self {
        Self::new_with_config_for_tests(
            genesis_config,
            AccountSecondaryIndexes::default(),
            false,
            AccountShrinkThreshold::default(),
        )
    }

    pub fn new_no_wallclock_throttle_for_tests(genesis_config: &GenesisConfig) -> Self {
        let mut bank = Self::new_for_tests(genesis_config);

        bank.ns_per_slot = std::u128::MAX;
        bank
    }

    pub(crate) fn new_with_config_for_tests(
        genesis_config: &GenesisConfig,
        account_indexes: AccountSecondaryIndexes,
        accounts_db_caching_enabled: bool,
        shrink_ratio: AccountShrinkThreshold,
    ) -> Self {
        Self::new_with_paths_for_tests(
            genesis_config,
            Vec::new(),
            None,
            None,
            account_indexes,
            accounts_db_caching_enabled,
            shrink_ratio,
            false,
            None,
        )
    }

    fn default_with_accounts(accounts: Accounts) -> Self {
        let mut bank = Self {
            rewrites_skipped_this_slot: Rewrites::default(),
            rc: BankRc::new(accounts, Slot::default()),
            status_cache: Arc::<RwLock<BankStatusCache>>::default(),
            blockhash_queue: RwLock::<BlockhashQueue>::default(),
            ancestors: Ancestors::default(),
            hash: RwLock::<Hash>::default(),
            parent_hash: Hash::default(),
            parent_slot: Slot::default(),
            hard_forks: Arc::<RwLock<HardForks>>::default(),
            transaction_count: AtomicU64::default(),
            transaction_error_count: AtomicU64::default(),
            transaction_entries_count: AtomicU64::default(),
            transactions_per_entry_max: AtomicU64::default(),
            tick_height: AtomicU64::default(),
            signature_count: AtomicU64::default(),
            capitalization: AtomicU64::default(),
            max_tick_height: u64::default(),
            hashes_per_tick: Option::<u64>::default(),
            ticks_per_slot: u64::default(),
            ns_per_slot: u128::default(),
            genesis_creation_time: UnixTimestamp::default(),
            slots_per_year: f64::default(),
            slot: Slot::default(),
            bank_id: BankId::default(),
            epoch: Epoch::default(),
            block_height: u64::default(),
            collector_id: Pubkey::default(),
            collector_fees: AtomicU64::default(),
            fee_calculator: FeeCalculator::default(),
            fee_rate_governor: FeeRateGovernor::default(),
            collected_rent: AtomicU64::default(),
            rent_collector: RentCollector::default(),
            epoch_schedule: EpochSchedule::default(),
            inflation: Arc::<RwLock<Inflation>>::default(),
            stakes_cache: StakesCache::default(),
            epoch_stakes: HashMap::<Epoch, EpochStakes>::default(),
            is_delta: AtomicBool::default(),
            builtin_programs: BuiltinPrograms::default(),
            compute_budget: Option::<ComputeBudget>::default(),
            builtin_feature_transitions: Arc::<Vec<BuiltinFeatureTransition>>::default(),
            rewards: RwLock::<Vec<(Pubkey, RewardInfo)>>::default(),
            cluster_type: Option::<ClusterType>::default(),
            lazy_rent_collection: AtomicBool::default(),
            rewards_pool_pubkeys: Arc::<HashSet<Pubkey>>::default(),
            cached_executors: RwLock::<CachedExecutors>::default(),
            transaction_debug_keys: Option::<Arc<HashSet<Pubkey>>>::default(),
            transaction_log_collector_config: Arc::<RwLock<TransactionLogCollectorConfig>>::default(
            ),
            transaction_log_collector: Arc::<RwLock<TransactionLogCollector>>::default(),
            feature_set: Arc::<FeatureSet>::default(),
            drop_callback: RwLock::new(OptionalDropCallback(None)),
            freeze_started: AtomicBool::default(),
            vote_only_bank: false,
            cost_tracker: RwLock::<CostTracker>::default(),
            sysvar_cache: RwLock::<SysvarCache>::default(),
            accounts_data_size_initial: 0,
            accounts_data_size_delta_on_chain: AtomicI64::new(0),
            accounts_data_size_delta_off_chain: AtomicI64::new(0),
            fee_structure: FeeStructure::default(),
        };

        let accounts_data_size_initial = bank.get_total_accounts_stats().unwrap().data_len as u64;
        bank.accounts_data_size_initial = accounts_data_size_initial;

        bank
    }

    pub fn new_with_paths_for_tests(
        genesis_config: &GenesisConfig,
        paths: Vec<PathBuf>,
        debug_keys: Option<Arc<HashSet<Pubkey>>>,
        additional_builtins: Option<&Builtins>,
        account_indexes: AccountSecondaryIndexes,
        accounts_db_caching_enabled: bool,
        shrink_ratio: AccountShrinkThreshold,
        debug_do_not_add_builtins: bool,
        accounts_db_config: Option<AccountsDbConfig>,
    ) -> Self {
        Self::new_with_paths(
            genesis_config,
            paths,
            debug_keys,
            additional_builtins,
            account_indexes,
            accounts_db_caching_enabled,
            shrink_ratio,
            debug_do_not_add_builtins,
            accounts_db_config.or(Some(ACCOUNTS_DB_CONFIG_FOR_TESTING)),
            None,
        )
    }

    pub fn new_with_paths_for_benches(
        genesis_config: &GenesisConfig,
        paths: Vec<PathBuf>,
        debug_keys: Option<Arc<HashSet<Pubkey>>>,
        additional_builtins: Option<&Builtins>,
        account_indexes: AccountSecondaryIndexes,
        accounts_db_caching_enabled: bool,
        shrink_ratio: AccountShrinkThreshold,
        debug_do_not_add_builtins: bool,
    ) -> Self {
        Self::new_with_paths(
            genesis_config,
            paths,
            debug_keys,
            additional_builtins,
            account_indexes,
            accounts_db_caching_enabled,
            shrink_ratio,
            debug_do_not_add_builtins,
            Some(ACCOUNTS_DB_CONFIG_FOR_BENCHMARKS),
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_with_paths(
        genesis_config: &GenesisConfig,
        paths: Vec<PathBuf>,
        debug_keys: Option<Arc<HashSet<Pubkey>>>,
        additional_builtins: Option<&Builtins>,
        account_indexes: AccountSecondaryIndexes,
        accounts_db_caching_enabled: bool,
        shrink_ratio: AccountShrinkThreshold,
        debug_do_not_add_builtins: bool,
        accounts_db_config: Option<AccountsDbConfig>,
        accounts_update_notifier: Option<AccountsUpdateNotifier>,
    ) -> Self {
        let accounts = Accounts::new_with_config(
            paths,
            &genesis_config.cluster_type,
            account_indexes,
            accounts_db_caching_enabled,
            shrink_ratio,
            accounts_db_config,
            accounts_update_notifier,
        );
        let mut bank = Self::default_with_accounts(accounts);
        bank.ancestors = Ancestors::from(vec![bank.slot()]);
        bank.transaction_debug_keys = debug_keys;
        bank.cluster_type = Some(genesis_config.cluster_type);

        bank.process_genesis_config(genesis_config);
        bank.finish_init(
            genesis_config,
            additional_builtins,
            debug_do_not_add_builtins,
        );

        // genesis needs stakes for all epochs up to the epoch implied by
        //  slot = 0 and genesis configuration
        {
            let stakes = bank.stakes_cache.stakes().clone();
            let stakes = Arc::new(StakesEnum::from(stakes));
            for epoch in 0..=bank.get_leader_schedule_epoch(bank.slot) {
                bank.epoch_stakes
                    .insert(epoch, EpochStakes::new(stakes.clone(), epoch));
            }
            bank.update_stake_history(None);
        }
        bank.update_clock(None);
        bank.update_rent();
        bank.update_epoch_schedule();
        bank.update_recent_blockhashes();
        bank.fill_missing_sysvar_cache_entries();
        bank
    }

    /// Create a new bank that points to an immutable checkpoint of another bank.
    pub fn new_from_parent(parent: &Arc<Bank>, collector_id: &Pubkey, slot: Slot) -> Self {
        Self::_new_from_parent(
            parent,
            collector_id,
            slot,
            null_tracer(),
            NewBankOptions::default(),
        )
    }

    pub fn new_from_parent_with_options(
        parent: &Arc<Bank>,
        collector_id: &Pubkey,
        slot: Slot,
        new_bank_options: NewBankOptions,
    ) -> Self {
        Self::_new_from_parent(parent, collector_id, slot, null_tracer(), new_bank_options)
    }

    pub fn new_from_parent_with_tracer(
        parent: &Arc<Bank>,
        collector_id: &Pubkey,
        slot: Slot,
        reward_calc_tracer: impl Fn(&RewardCalculationEvent) + Send + Sync,
    ) -> Self {
        Self::_new_from_parent(
            parent,
            collector_id,
            slot,
            Some(reward_calc_tracer),
            NewBankOptions::default(),
        )
    }

    fn get_rent_collector_from(rent_collector: &RentCollector, epoch: Epoch) -> RentCollector {
        rent_collector.clone_with_epoch(epoch)
    }

    fn _new_from_parent(
        parent: &Arc<Bank>,
        collector_id: &Pubkey,
        slot: Slot,
        reward_calc_tracer: Option<impl Fn(&RewardCalculationEvent) + Send + Sync>,
        new_bank_options: NewBankOptions,
    ) -> Self {
        let mut time = Measure::start("bank::new_from_parent");
        let NewBankOptions { vote_only_bank } = new_bank_options;

        parent.freeze();
        assert_ne!(slot, parent.slot());

        let epoch_schedule = parent.epoch_schedule;
        let epoch = epoch_schedule.get_epoch(slot);

        let (rc, bank_rc_time) = measure!(
            BankRc {
                accounts: Arc::new(Accounts::new_from_parent(
                    &parent.rc.accounts,
                    slot,
                    parent.slot(),
                )),
                parent: RwLock::new(Some(parent.clone())),
                slot,
                bank_id_generator: parent.rc.bank_id_generator.clone(),
            },
            "bank_rc_creation",
        );

        let (status_cache, status_cache_time) =
            measure!(Arc::clone(&parent.status_cache), "status_cache_creation",);

        let ((fee_rate_governor, fee_calculator), fee_components_time) = measure!(
            {
                let fee_rate_governor = FeeRateGovernor::new_derived(
                    &parent.fee_rate_governor,
                    parent.signature_count(),
                );

                let fee_calculator = if parent.feature_set.is_active(&disable_fee_calculator::id())
                {
                    FeeCalculator::default()
                } else {
                    fee_rate_governor.create_fee_calculator()
                };
                (fee_rate_governor, fee_calculator)
            },
            "fee_components_creation",
        );

        let bank_id = rc.bank_id_generator.fetch_add(1, Relaxed) + 1;
        let (blockhash_queue, blockhash_queue_time) = measure!(
            RwLock::new(parent.blockhash_queue.read().unwrap().clone()),
            "blockhash_queue_creation",
        );

        let (stakes_cache, stakes_cache_time) = measure!(
            StakesCache::new(parent.stakes_cache.stakes().clone()),
            "stakes_cache_creation",
        );

        let (epoch_stakes, epoch_stakes_time) =
            measure!(parent.epoch_stakes.clone(), "epoch_stakes_creation");

        let (builtin_programs, builtin_programs_time) =
            measure!(parent.builtin_programs.clone(), "builtin_programs_creation");

        let (rewards_pool_pubkeys, rewards_pool_pubkeys_time) = measure!(
            parent.rewards_pool_pubkeys.clone(),
            "rewards_pool_pubkeys_creation",
        );

        let (cached_executors, cached_executors_time) = measure!(
            {
                let parent_bank_executors = parent.cached_executors.read().unwrap();
                RwLock::new(CachedExecutors::new_from_parent_bank_executors(
                    &parent_bank_executors,
                    epoch,
                ))
            },
            "cached_executors_creation",
        );

        let (transaction_debug_keys, transaction_debug_keys_time) = measure!(
            parent.transaction_debug_keys.clone(),
            "transation_debug_keys_creation",
        );

        let (transaction_log_collector_config, transaction_log_collector_config_time) = measure!(
            parent.transaction_log_collector_config.clone(),
            "transaction_log_collector_config_creation",
        );

        let (feature_set, feature_set_time) =
            measure!(parent.feature_set.clone(), "feature_set_creation");

        let accounts_data_size_initial = parent.load_accounts_data_size();
        let mut new = Bank {
            rewrites_skipped_this_slot: Rewrites::default(),
            rc,
            status_cache,
            slot,
            bank_id,
            epoch,
            blockhash_queue,

            // TODO: clean this up, so much special-case copying...
            hashes_per_tick: parent.hashes_per_tick,
            ticks_per_slot: parent.ticks_per_slot,
            ns_per_slot: parent.ns_per_slot,
            genesis_creation_time: parent.genesis_creation_time,
            slots_per_year: parent.slots_per_year,
            epoch_schedule,
            collected_rent: AtomicU64::new(0),
            rent_collector: Self::get_rent_collector_from(&parent.rent_collector, epoch),
            max_tick_height: (slot + 1) * parent.ticks_per_slot,
            block_height: parent.block_height + 1,
            fee_calculator,
            fee_rate_governor,
            capitalization: AtomicU64::new(parent.capitalization()),
            vote_only_bank,
            inflation: parent.inflation.clone(),
            transaction_count: AtomicU64::new(parent.transaction_count()),
            transaction_error_count: AtomicU64::new(0),
            transaction_entries_count: AtomicU64::new(0),
            transactions_per_entry_max: AtomicU64::new(0),
            // we will .clone_with_epoch() this soon after stake data update; so just .clone() for now
            stakes_cache,
            epoch_stakes,
            parent_hash: parent.hash(),
            parent_slot: parent.slot(),
            collector_id: *collector_id,
            collector_fees: AtomicU64::new(0),
            ancestors: Ancestors::default(),
            hash: RwLock::new(Hash::default()),
            is_delta: AtomicBool::new(false),
            tick_height: AtomicU64::new(parent.tick_height.load(Relaxed)),
            signature_count: AtomicU64::new(0),
            builtin_programs,
            compute_budget: parent.compute_budget,
            builtin_feature_transitions: parent.builtin_feature_transitions.clone(),
            hard_forks: parent.hard_forks.clone(),
            rewards: RwLock::new(vec![]),
            cluster_type: parent.cluster_type,
            lazy_rent_collection: AtomicBool::new(parent.lazy_rent_collection.load(Relaxed)),
            rewards_pool_pubkeys,
            cached_executors,
            transaction_debug_keys,
            transaction_log_collector_config,
            transaction_log_collector: Arc::new(RwLock::new(TransactionLogCollector::default())),
            feature_set: Arc::clone(&feature_set),
            drop_callback: RwLock::new(OptionalDropCallback(
                parent
                    .drop_callback
                    .read()
                    .unwrap()
                    .0
                    .as_ref()
                    .map(|drop_callback| drop_callback.clone_box()),
            )),
            freeze_started: AtomicBool::new(false),
            cost_tracker: RwLock::new(CostTracker::new_with_account_data_size_limit(
                feature_set
                    .is_active(&feature_set::cap_accounts_data_len::id())
                    .then(|| {
                        parent
                            .accounts_data_size_limit()
                            .saturating_sub(accounts_data_size_initial)
                    }),
            )),
            sysvar_cache: RwLock::new(SysvarCache::default()),
            accounts_data_size_initial,
            accounts_data_size_delta_on_chain: AtomicI64::new(0),
            accounts_data_size_delta_off_chain: AtomicI64::new(0),
            fee_structure: parent.fee_structure.clone(),
        };

        let (_, ancestors_time) = measure!(
            {
                let mut ancestors = Vec::with_capacity(1 + new.parents().len());
                ancestors.push(new.slot());
                new.parents().iter().for_each(|p| {
                    ancestors.push(p.slot());
                });
                new.ancestors = Ancestors::from(ancestors);
            },
            "ancestors_creation",
        );

        // Following code may touch AccountsDb, requiring proper ancestors
        let parent_epoch = parent.epoch();
        let (_, update_epoch_time) = measure!(
            {
                if parent_epoch < new.epoch() {
                    let (thread_pool, thread_pool_time) = measure!(
                        ThreadPoolBuilder::new().build().unwrap(),
                        "thread_pool_creation",
                    );

                    let (_, apply_feature_activations_time) = measure!(
                        new.apply_feature_activations(
                            ApplyFeatureActivationsCaller::NewFromParent,
                            false
                        ),
                        "apply_feature_activation",
                    );

                    // Add new entry to stakes.stake_history, set appropriate epoch and
                    // update vote accounts with warmed up stakes before saving a
                    // snapshot of stakes in epoch stakes
                    let (_, activate_epoch_time) = measure!(
                        new.stakes_cache.activate_epoch(epoch, &thread_pool),
                        "activate_epoch",
                    );

                    // Save a snapshot of stakes for use in consensus and stake weighted networking
                    let leader_schedule_epoch = epoch_schedule.get_leader_schedule_epoch(slot);
                    let (_, update_epoch_stakes_time) = measure!(
                        new.update_epoch_stakes(leader_schedule_epoch),
                        "update_epoch_stakes",
                    );

                    let mut metrics = RewardsMetrics::default();
                    // After saving a snapshot of stakes, apply stake rewards and commission
                    let (_, update_rewards_with_thread_pool_time) = measure!(
                        {
                            new.update_rewards_with_thread_pool(
                                parent_epoch,
                                reward_calc_tracer,
                                &thread_pool,
                                &mut metrics,
                            )
                        },
                        "update_rewards_with_thread_pool",
                    );

                    datapoint_info!(
                        "bank-new_from_parent-new_epoch_timings",
                        ("epoch", new.epoch(), i64),
                        ("slot", slot, i64),
                        ("parent_slot", parent.slot(), i64),
                        ("thread_pool_creation_us", thread_pool_time.as_us(), i64),
                        (
                            "apply_feature_activations",
                            apply_feature_activations_time.as_us(),
                            i64
                        ),
                        ("activate_epoch_Us", activate_epoch_time.as_us(), i64),
                        (
                            "update_epoch_stakes_us",
                            update_epoch_stakes_time.as_us(),
                            i64
                        ),
                        (
                            "update_rewards_with_thread_pool_us",
                            update_rewards_with_thread_pool_time.as_us(),
                            i64
                        ),
                        (
                            "load_vote_and_stake_accounts_us",
                            metrics.load_vote_and_stake_accounts_us.load(Relaxed),
                            i64
                        ),
                        (
                            "calculate_points_us",
                            metrics.calculate_points_us.load(Relaxed),
                            i64
                        ),
                        ("redeem_rewards_us", metrics.redeem_rewards_us, i64),
                        (
                            "store_stake_accounts_us",
                            metrics.store_stake_accounts_us.load(Relaxed),
                            i64
                        ),
                        (
                            "store_vote_accounts_us",
                            metrics.store_vote_accounts_us.load(Relaxed),
                            i64
                        ),
                        (
                            "invalid_cached_vote_accounts",
                            metrics.invalid_cached_vote_accounts,
                            i64
                        ),
                        (
                            "invalid_cached_stake_accounts",
                            metrics.invalid_cached_stake_accounts,
                            i64
                        ),
                        (
                            "invalid_cached_stake_accounts_rent_epoch",
                            metrics.invalid_cached_stake_accounts_rent_epoch,
                            i64
                        ),
                        (
                            "vote_accounts_cache_miss_count",
                            metrics.vote_accounts_cache_miss_count,
                            i64
                        ),
                    );
                } else {
                    // Save a snapshot of stakes for use in consensus and stake weighted networking
                    let leader_schedule_epoch = epoch_schedule.get_leader_schedule_epoch(slot);
                    new.update_epoch_stakes(leader_schedule_epoch);
                }
            },
            "update_epoch",
        );

        // Update sysvars before processing transactions
        let (_, update_sysvars_time) = measure!(
            {
                new.update_slot_hashes();
                new.update_stake_history(Some(parent_epoch));
                new.update_clock(Some(parent_epoch));
                new.update_fees();
            },
            "update_sysvars",
        );

        let (_, fill_sysvar_cache_time) =
            measure!(new.fill_missing_sysvar_cache_entries(), "fill_sysvar_cache");

        time.stop();

        datapoint_info!(
            "bank-new_from_parent-heights",
            ("slot", slot, i64),
            ("block_height", new.block_height, i64),
            ("parent_slot", parent.slot(), i64),
            ("bank_rc_creation_us", bank_rc_time.as_us(), i64),
            ("total_elapsed_us", time.as_us(), i64),
            ("status_cache_us", status_cache_time.as_us(), i64),
            ("fee_components_us", fee_components_time.as_us(), i64),
            ("blockhash_queue_us", blockhash_queue_time.as_us(), i64),
            ("stakes_cache_us", stakes_cache_time.as_us(), i64),
            ("epoch_stakes_time_us", epoch_stakes_time.as_us(), i64),
            ("builtin_programs_us", builtin_programs_time.as_us(), i64),
            (
                "rewards_pool_pubkeys_us",
                rewards_pool_pubkeys_time.as_us(),
                i64
            ),
            ("cached_executors_us", cached_executors_time.as_us(), i64),
            (
                "transaction_debug_keys_us",
                transaction_debug_keys_time.as_us(),
                i64
            ),
            (
                "transaction_log_collector_config_us",
                transaction_log_collector_config_time.as_us(),
                i64
            ),
            ("feature_set_us", feature_set_time.as_us(), i64),
            ("ancestors_us", ancestors_time.as_us(), i64),
            ("update_epoch_us", update_epoch_time.as_us(), i64),
            ("update_sysvars_us", update_sysvars_time.as_us(), i64),
            ("fill_sysvar_cache_us", fill_sysvar_cache_time.as_us(), i64),
        );

        parent
            .cached_executors
            .read()
            .unwrap()
            .stats
            .submit(parent.slot());

        new
    }

    pub fn byte_limit_for_scans(&self) -> Option<usize> {
        self.rc
            .accounts
            .accounts_db
            .accounts_index
            .scan_results_limit_bytes
    }

    pub fn proper_ancestors_set(&self) -> HashSet<Slot> {
        HashSet::from_iter(self.proper_ancestors())
    }

    /// Returns all ancestors excluding self.slot.
    pub(crate) fn proper_ancestors(&self) -> impl Iterator<Item = Slot> + '_ {
        self.ancestors
            .keys()
            .into_iter()
            .filter(move |slot| *slot != self.slot)
    }

    pub fn set_callback(&self, callback: Option<Box<dyn DropCallback + Send + Sync>>) {
        *self.drop_callback.write().unwrap() = OptionalDropCallback(callback);
    }

    pub fn vote_only_bank(&self) -> bool {
        self.vote_only_bank
    }

    /// Like `new_from_parent` but additionally:
    /// * Doesn't assume that the parent is anywhere near `slot`, parent could be millions of slots
    /// in the past
    /// * Adjusts the new bank's tick height to avoid having to run PoH for millions of slots
    /// * Freezes the new bank, assuming that the user will `Bank::new_from_parent` from this bank
    pub fn warp_from_parent(parent: &Arc<Bank>, collector_id: &Pubkey, slot: Slot) -> Self {
        let parent_timestamp = parent.clock().unix_timestamp;
        let mut new = Bank::new_from_parent(parent, collector_id, slot);
        new.apply_feature_activations(ApplyFeatureActivationsCaller::WarpFromParent, false);
        new.update_epoch_stakes(new.epoch_schedule().get_epoch(slot));
        new.tick_height.store(new.max_tick_height(), Relaxed);

        let mut clock = new.clock();
        clock.epoch_start_timestamp = parent_timestamp;
        clock.unix_timestamp = parent_timestamp;
        new.update_sysvar_account(&sysvar::clock::id(), |account| {
            create_account(
                &clock,
                new.inherit_specially_retained_account_fields(account),
            )
        });
        new.fill_missing_sysvar_cache_entries();
        new.freeze();
        new
    }

    /// Create a bank from explicit arguments and deserialized fields from snapshot
    #[allow(clippy::float_cmp)]
    pub(crate) fn new_from_fields(
        bank_rc: BankRc,
        genesis_config: &GenesisConfig,
        fields: BankFieldsToDeserialize,
        debug_keys: Option<Arc<HashSet<Pubkey>>>,
        additional_builtins: Option<&Builtins>,
        debug_do_not_add_builtins: bool,
        accounts_data_size_initial: u64,
    ) -> Self {
        let now = Instant::now();
        let ancestors = Ancestors::from(&fields.ancestors);
        // For backward compatibility, we can only serialize and deserialize
        // Stakes<Delegation> in BankFieldsTo{Serialize,Deserialize}. But Bank
        // caches Stakes<StakeAccount>. Below Stakes<StakeAccount> is obtained
        // from Stakes<Delegation> by reading the full account state from
        // accounts-db. Note that it is crucial that these accounts are loaded
        // at the right slot and match precisely with serialized Delegations.
        let stakes = Stakes::new(&fields.stakes, |pubkey| {
            let (account, _slot) = bank_rc.accounts.load_with_fixed_root(&ancestors, pubkey)?;
            Some(account)
        })
        .expect(
            "Stakes cache is inconsistent with accounts-db. This can indicate \
            a corrupted snapshot or bugs in cached accounts or accounts-db.",
        );
        let stakes_accounts_load_duration = now.elapsed();
        fn new<T: Default>() -> T {
            T::default()
        }
        let feature_set = new();
        let mut bank = Self {
            rewrites_skipped_this_slot: Rewrites::default(),
            rc: bank_rc,
            status_cache: new(),
            blockhash_queue: RwLock::new(fields.blockhash_queue),
            ancestors,
            hash: RwLock::new(fields.hash),
            parent_hash: fields.parent_hash,
            parent_slot: fields.parent_slot,
            hard_forks: Arc::new(RwLock::new(fields.hard_forks)),
            transaction_count: AtomicU64::new(fields.transaction_count),
            transaction_error_count: new(),
            transaction_entries_count: new(),
            transactions_per_entry_max: new(),
            tick_height: AtomicU64::new(fields.tick_height),
            signature_count: AtomicU64::new(fields.signature_count),
            capitalization: AtomicU64::new(fields.capitalization),
            max_tick_height: fields.max_tick_height,
            hashes_per_tick: fields.hashes_per_tick,
            ticks_per_slot: fields.ticks_per_slot,
            ns_per_slot: fields.ns_per_slot,
            genesis_creation_time: fields.genesis_creation_time,
            slots_per_year: fields.slots_per_year,
            slot: fields.slot,
            bank_id: 0,
            epoch: fields.epoch,
            block_height: fields.block_height,
            collector_id: fields.collector_id,
            collector_fees: AtomicU64::new(fields.collector_fees),
            fee_calculator: fields.fee_calculator,
            fee_rate_governor: fields.fee_rate_governor,
            collected_rent: AtomicU64::new(fields.collected_rent),
            // clone()-ing is needed to consider a gated behavior in rent_collector
            rent_collector: Self::get_rent_collector_from(&fields.rent_collector, fields.epoch),
            epoch_schedule: fields.epoch_schedule,
            inflation: Arc::new(RwLock::new(fields.inflation)),
            stakes_cache: StakesCache::new(stakes),
            epoch_stakes: fields.epoch_stakes,
            is_delta: AtomicBool::new(fields.is_delta),
            builtin_programs: new(),
            compute_budget: None,
            builtin_feature_transitions: new(),
            rewards: new(),
            cluster_type: Some(genesis_config.cluster_type),
            lazy_rent_collection: new(),
            rewards_pool_pubkeys: new(),
            cached_executors: RwLock::new(CachedExecutors::new(MAX_CACHED_EXECUTORS, fields.epoch)),
            transaction_debug_keys: debug_keys,
            transaction_log_collector_config: new(),
            transaction_log_collector: new(),
            feature_set: Arc::clone(&feature_set),
            drop_callback: RwLock::new(OptionalDropCallback(None)),
            freeze_started: AtomicBool::new(fields.hash != Hash::default()),
            vote_only_bank: false,
            cost_tracker: RwLock::new(CostTracker::default()),
            sysvar_cache: RwLock::new(SysvarCache::default()),
            accounts_data_size_initial,
            accounts_data_size_delta_on_chain: AtomicI64::new(0),
            accounts_data_size_delta_off_chain: AtomicI64::new(0),
            fee_structure: FeeStructure::default(),
        };
        bank.finish_init(
            genesis_config,
            additional_builtins,
            debug_do_not_add_builtins,
        );

        // Sanity assertions between bank snapshot and genesis config
        // Consider removing from serializable bank state
        // (BankFieldsToSerialize/BankFieldsToDeserialize) and initializing
        // from the passed in genesis_config instead (as new()/new_with_paths() already do)
        assert_eq!(
            bank.hashes_per_tick,
            genesis_config.poh_config.hashes_per_tick
        );
        assert_eq!(bank.ticks_per_slot, genesis_config.ticks_per_slot);
        assert_eq!(
            bank.ns_per_slot,
            genesis_config.poh_config.target_tick_duration.as_nanos()
                * genesis_config.ticks_per_slot as u128
        );
        assert_eq!(bank.genesis_creation_time, genesis_config.creation_time);
        assert_eq!(bank.max_tick_height, (bank.slot + 1) * bank.ticks_per_slot);
        assert_eq!(
            bank.slots_per_year,
            years_as_slots(
                1.0,
                &genesis_config.poh_config.target_tick_duration,
                bank.ticks_per_slot,
            )
        );
        assert_eq!(bank.epoch_schedule, genesis_config.epoch_schedule);
        assert_eq!(bank.epoch, bank.epoch_schedule.get_epoch(bank.slot));
        if !bank.feature_set.is_active(&disable_fee_calculator::id()) {
            bank.fee_rate_governor.lamports_per_signature =
                bank.fee_calculator.lamports_per_signature;
            assert_eq!(
                bank.fee_rate_governor.create_fee_calculator(),
                bank.fee_calculator
            );
        }

        datapoint_info!(
            "bank-new-from-fields",
            (
                "accounts_data_len-from-snapshot",
                fields.accounts_data_len as i64,
                i64
            ),
            (
                "accounts_data_len-from-generate_index",
                accounts_data_size_initial as i64,
                i64
            ),
            (
                "stakes_accounts_load_duration_us",
                stakes_accounts_load_duration.as_micros(),
                i64
            ),
        );
        bank
    }

    /// Return subset of bank fields representing serializable state
    pub(crate) fn get_fields_to_serialize<'a>(
        &'a self,
        ancestors: &'a HashMap<Slot, usize>,
    ) -> BankFieldsToSerialize<'a> {
        BankFieldsToSerialize {
            blockhash_queue: &self.blockhash_queue,
            ancestors,
            hash: *self.hash.read().unwrap(),
            parent_hash: self.parent_hash,
            parent_slot: self.parent_slot,
            hard_forks: &*self.hard_forks,
            transaction_count: self.transaction_count.load(Relaxed),
            tick_height: self.tick_height.load(Relaxed),
            signature_count: self.signature_count.load(Relaxed),
            capitalization: self.capitalization.load(Relaxed),
            max_tick_height: self.max_tick_height,
            hashes_per_tick: self.hashes_per_tick,
            ticks_per_slot: self.ticks_per_slot,
            ns_per_slot: self.ns_per_slot,
            genesis_creation_time: self.genesis_creation_time,
            slots_per_year: self.slots_per_year,
            slot: self.slot,
            epoch: self.epoch,
            block_height: self.block_height,
            collector_id: self.collector_id,
            collector_fees: self.collector_fees.load(Relaxed),
            fee_calculator: self.fee_calculator,
            fee_rate_governor: self.fee_rate_governor.clone(),
            collected_rent: self.collected_rent.load(Relaxed),
            rent_collector: self.rent_collector.clone(),
            epoch_schedule: self.epoch_schedule,
            inflation: *self.inflation.read().unwrap(),
            stakes: &self.stakes_cache,
            epoch_stakes: &self.epoch_stakes,
            is_delta: self.is_delta.load(Relaxed),
            accounts_data_len: self.load_accounts_data_size(),
        }
    }

    pub fn collector_id(&self) -> &Pubkey {
        &self.collector_id
    }

    pub fn genesis_creation_time(&self) -> UnixTimestamp {
        self.genesis_creation_time
    }

    pub fn slot(&self) -> Slot {
        self.slot
    }

    pub fn bank_id(&self) -> BankId {
        self.bank_id
    }

    pub fn epoch(&self) -> Epoch {
        self.epoch
    }

    pub fn first_normal_epoch(&self) -> Epoch {
        self.epoch_schedule().first_normal_epoch
    }

    pub fn freeze_lock(&self) -> RwLockReadGuard<Hash> {
        self.hash.read().unwrap()
    }

    pub fn hash(&self) -> Hash {
        *self.hash.read().unwrap()
    }

    pub fn is_frozen(&self) -> bool {
        *self.hash.read().unwrap() != Hash::default()
    }

    pub fn freeze_started(&self) -> bool {
        self.freeze_started.load(Relaxed)
    }

    pub fn status_cache_ancestors(&self) -> Vec<u64> {
        let mut roots = self.status_cache.read().unwrap().roots().clone();
        let min = roots.iter().min().cloned().unwrap_or(0);
        for ancestor in self.ancestors.keys() {
            if ancestor >= min {
                roots.insert(ancestor);
            }
        }

        let mut ancestors: Vec<_> = roots.into_iter().collect();
        #[allow(clippy::stable_sort_primitive)]
        ancestors.sort();
        ancestors
    }

    /// computed unix_timestamp at this slot height
    pub fn unix_timestamp_from_genesis(&self) -> i64 {
        self.genesis_creation_time + ((self.slot as u128 * self.ns_per_slot) / 1_000_000_000) as i64
    }

    fn update_sysvar_account<F>(&self, pubkey: &Pubkey, updater: F)
    where
        F: Fn(&Option<AccountSharedData>) -> AccountSharedData,
    {
        let old_account = self.get_account_with_fixed_root(pubkey);
        let mut new_account = updater(&old_account);

        // When new sysvar comes into existence (with RENT_UNADJUSTED_INITIAL_BALANCE lamports),
        // this code ensures that the sysvar's balance is adjusted to be rent-exempt.
        //
        // More generally, this code always re-calculates for possible sysvar data size change,
        // although there is no such sysvars currently.
        self.adjust_sysvar_balance_for_rent(&mut new_account);
        self.store_account_and_update_capitalization(pubkey, &new_account);
    }

    fn inherit_specially_retained_account_fields(
        &self,
        old_account: &Option<AccountSharedData>,
    ) -> InheritableAccountFields {
        const RENT_UNADJUSTED_INITIAL_BALANCE: u64 = 1;

        (
            old_account
                .as_ref()
                .map(|a| a.lamports())
                .unwrap_or(RENT_UNADJUSTED_INITIAL_BALANCE),
            old_account
                .as_ref()
                .map(|a| a.rent_epoch())
                .unwrap_or(INITIAL_RENT_EPOCH),
        )
    }

    pub fn clock(&self) -> sysvar::clock::Clock {
        from_account(&self.get_account(&sysvar::clock::id()).unwrap_or_default())
            .unwrap_or_default()
    }

    fn update_clock(&self, parent_epoch: Option<Epoch>) {
        let mut unix_timestamp = self.clock().unix_timestamp;
        // set epoch_start_timestamp to None to warp timestamp
        let epoch_start_timestamp = {
            let epoch = if let Some(epoch) = parent_epoch {
                epoch
            } else {
                self.epoch()
            };
            let first_slot_in_epoch = self.epoch_schedule().get_first_slot_in_epoch(epoch);
            Some((first_slot_in_epoch, self.clock().epoch_start_timestamp))
        };
        let max_allowable_drift = MaxAllowableDrift {
            fast: MAX_ALLOWABLE_DRIFT_PERCENTAGE_FAST,
            slow: MAX_ALLOWABLE_DRIFT_PERCENTAGE_SLOW_V2,
        };

        let ancestor_timestamp = self.clock().unix_timestamp;
        if let Some(timestamp_estimate) =
            self.get_timestamp_estimate(max_allowable_drift, epoch_start_timestamp)
        {
            unix_timestamp = timestamp_estimate;
            if timestamp_estimate < ancestor_timestamp {
                unix_timestamp = ancestor_timestamp;
            }
        }
        datapoint_info!(
            "bank-timestamp-correction",
            ("slot", self.slot(), i64),
            ("from_genesis", self.unix_timestamp_from_genesis(), i64),
            ("corrected", unix_timestamp, i64),
            ("ancestor_timestamp", ancestor_timestamp, i64),
        );
        let mut epoch_start_timestamp =
            // On epoch boundaries, update epoch_start_timestamp
            if parent_epoch.is_some() && parent_epoch.unwrap() != self.epoch() {
                unix_timestamp
            } else {
                self.clock().epoch_start_timestamp
            };
        if self.slot == 0 {
            unix_timestamp = self.unix_timestamp_from_genesis();
            epoch_start_timestamp = self.unix_timestamp_from_genesis();
        }
        let clock = sysvar::clock::Clock {
            slot: self.slot,
            epoch_start_timestamp,
            epoch: self.epoch_schedule().get_epoch(self.slot),
            leader_schedule_epoch: self.epoch_schedule().get_leader_schedule_epoch(self.slot),
            unix_timestamp,
        };
        self.update_sysvar_account(&sysvar::clock::id(), |account| {
            create_account(
                &clock,
                self.inherit_specially_retained_account_fields(account),
            )
        });
    }

    pub fn set_sysvar_for_tests<T>(&self, sysvar: &T)
    where
        T: Sysvar + SysvarId,
    {
        self.update_sysvar_account(&T::id(), |account| {
            create_account(
                sysvar,
                self.inherit_specially_retained_account_fields(account),
            )
        });
        // Simply force fill sysvar cache rather than checking which sysvar was
        // actually updated since tests don't need to be optimized for performance.
        self.reset_sysvar_cache();
        self.fill_missing_sysvar_cache_entries();
    }

    fn update_slot_history(&self) {
        self.update_sysvar_account(&sysvar::slot_history::id(), |account| {
            let mut slot_history = account
                .as_ref()
                .map(|account| from_account::<SlotHistory, _>(account).unwrap())
                .unwrap_or_default();
            slot_history.add(self.slot());
            create_account(
                &slot_history,
                self.inherit_specially_retained_account_fields(account),
            )
        });
    }

    fn update_slot_hashes(&self) {
        self.update_sysvar_account(&sysvar::slot_hashes::id(), |account| {
            let mut slot_hashes = account
                .as_ref()
                .map(|account| from_account::<SlotHashes, _>(account).unwrap())
                .unwrap_or_default();
            slot_hashes.add(self.parent_slot, self.parent_hash);
            create_account(
                &slot_hashes,
                self.inherit_specially_retained_account_fields(account),
            )
        });
    }

    pub fn get_slot_history(&self) -> SlotHistory {
        from_account(&self.get_account(&sysvar::slot_history::id()).unwrap()).unwrap()
    }

    fn update_epoch_stakes(&mut self, leader_schedule_epoch: Epoch) {
        // update epoch_stakes cache
        //  if my parent didn't populate for this staker's epoch, we've
        //  crossed a boundary
        if self.epoch_stakes.get(&leader_schedule_epoch).is_none() {
            self.epoch_stakes.retain(|&epoch, _| {
                epoch >= leader_schedule_epoch.saturating_sub(MAX_LEADER_SCHEDULE_STAKES)
            });
            let stakes = self.stakes_cache.stakes().clone();
            let stakes = Arc::new(StakesEnum::from(stakes));
            let new_epoch_stakes = EpochStakes::new(stakes, leader_schedule_epoch);
            {
                let vote_stakes: HashMap<_, _> = self
                    .stakes_cache
                    .stakes()
                    .vote_accounts()
                    .delegated_stakes()
                    .map(|(pubkey, stake)| (*pubkey, stake))
                    .collect();
                info!(
                    "new epoch stakes, epoch: {}, stakes: {:#?}, total_stake: {}",
                    leader_schedule_epoch,
                    vote_stakes,
                    new_epoch_stakes.total_stake(),
                );
            }
            self.epoch_stakes
                .insert(leader_schedule_epoch, new_epoch_stakes);
        }
    }

    #[allow(deprecated)]
    fn update_fees(&self) {
        if !self
            .feature_set
            .is_active(&feature_set::disable_fees_sysvar::id())
        {
            self.update_sysvar_account(&sysvar::fees::id(), |account| {
                create_account(
                    &sysvar::fees::Fees::new(&self.fee_rate_governor.create_fee_calculator()),
                    self.inherit_specially_retained_account_fields(account),
                )
            });
        }
    }

    fn update_rent(&self) {
        self.update_sysvar_account(&sysvar::rent::id(), |account| {
            create_account(
                &self.rent_collector.rent,
                self.inherit_specially_retained_account_fields(account),
            )
        });
    }

    fn update_epoch_schedule(&self) {
        self.update_sysvar_account(&sysvar::epoch_schedule::id(), |account| {
            create_account(
                self.epoch_schedule(),
                self.inherit_specially_retained_account_fields(account),
            )
        });
    }

    fn update_stake_history(&self, epoch: Option<Epoch>) {
        if epoch == Some(self.epoch()) {
            return;
        }
        // if I'm the first Bank in an epoch, ensure stake_history is updated
        self.update_sysvar_account(&sysvar::stake_history::id(), |account| {
            create_account::<sysvar::stake_history::StakeHistory>(
                self.stakes_cache.stakes().history(),
                self.inherit_specially_retained_account_fields(account),
            )
        });
    }

    pub fn epoch_duration_in_years(&self, prev_epoch: Epoch) -> f64 {
        // period: time that has passed as a fraction of a year, basically the length of
        //  an epoch as a fraction of a year
        //  calculated as: slots_elapsed / (slots / year)
        self.epoch_schedule().get_slots_in_epoch(prev_epoch) as f64 / self.slots_per_year
    }

    // Calculates the starting-slot for inflation from the activation slot.
    // This method assumes that `pico_inflation` will be enabled before `full_inflation`, giving
    // precedence to the latter. However, since `pico_inflation` is fixed-rate Inflation, should
    // `pico_inflation` be enabled 2nd, the incorrect start slot provided here should have no
    // effect on the inflation calculation.
    fn get_inflation_start_slot(&self) -> Slot {
        let mut slots = self
            .feature_set
            .full_inflation_features_enabled()
            .iter()
            .filter_map(|id| self.feature_set.activated_slot(id))
            .collect::<Vec<_>>();
        slots.sort_unstable();
        slots.first().cloned().unwrap_or_else(|| {
            self.feature_set
                .activated_slot(&feature_set::pico_inflation::id())
                .unwrap_or(0)
        })
    }

    fn get_inflation_num_slots(&self) -> u64 {
        let inflation_activation_slot = self.get_inflation_start_slot();
        // Normalize inflation_start to align with the start of rewards accrual.
        let inflation_start_slot = self.epoch_schedule().get_first_slot_in_epoch(
            self.epoch_schedule()
                .get_epoch(inflation_activation_slot)
                .saturating_sub(1),
        );
        self.epoch_schedule().get_first_slot_in_epoch(self.epoch()) - inflation_start_slot
    }

    pub fn slot_in_year_for_inflation(&self) -> f64 {
        let num_slots = self.get_inflation_num_slots();

        // calculated as: num_slots / (slots / year)
        num_slots as f64 / self.slots_per_year
    }

    fn calculate_previous_epoch_inflation_rewards(
        &self,
        prev_epoch_capitalization: u64,
        prev_epoch: Epoch,
    ) -> PrevEpochInflationRewards {
        let slot_in_year = self.slot_in_year_for_inflation();
        let (validator_rate, foundation_rate) = {
            let inflation = self.inflation.read().unwrap();
            (
                (*inflation).validator(slot_in_year),
                (*inflation).foundation(slot_in_year),
            )
        };

        let prev_epoch_duration_in_years = self.epoch_duration_in_years(prev_epoch);
        let validator_rewards = (validator_rate
            * prev_epoch_capitalization as f64
            * prev_epoch_duration_in_years) as u64;

        PrevEpochInflationRewards {
            validator_rewards,
            prev_epoch_duration_in_years,
            validator_rate,
            foundation_rate,
        }
    }

    // update rewards based on the previous epoch
    fn update_rewards_with_thread_pool(
        &mut self,
        prev_epoch: Epoch,
        reward_calc_tracer: Option<impl Fn(&RewardCalculationEvent) + Send + Sync>,
        thread_pool: &ThreadPool,
        metrics: &mut RewardsMetrics,
    ) {
        let capitalization = self.capitalization();
        let PrevEpochInflationRewards {
            validator_rewards,
            prev_epoch_duration_in_years,
            validator_rate,
            foundation_rate,
        } = self.calculate_previous_epoch_inflation_rewards(capitalization, prev_epoch);

        let old_vote_balance_and_staked = self.stakes_cache.stakes().vote_balance_and_staked();
        let update_rewards_from_cached_accounts = self
            .feature_set
            .is_active(&feature_set::update_rewards_from_cached_accounts::id());

        self.pay_validator_rewards_with_thread_pool(
            prev_epoch,
            validator_rewards,
            reward_calc_tracer,
            self.credits_auto_rewind(),
            thread_pool,
            metrics,
            update_rewards_from_cached_accounts,
        );

        let new_vote_balance_and_staked = self.stakes_cache.stakes().vote_balance_and_staked();
        let validator_rewards_paid = new_vote_balance_and_staked - old_vote_balance_and_staked;
        assert_eq!(
            validator_rewards_paid,
            u64::try_from(
                self.rewards
                    .read()
                    .unwrap()
                    .iter()
                    .map(|(_address, reward_info)| {
                        match reward_info.reward_type {
                            RewardType::Voting | RewardType::Staking => reward_info.lamports,
                            _ => 0,
                        }
                    })
                    .sum::<i64>()
            )
            .unwrap()
        );

        // verify that we didn't pay any more than we expected to
        assert!(validator_rewards >= validator_rewards_paid);

        info!(
            "distributed inflation: {} (rounded from: {})",
            validator_rewards_paid, validator_rewards
        );
        // TODO: staked_nodes forces an eager stakes calculation. remove it!
        let (num_stake_accounts, num_vote_accounts, num_staked_nodes) = {
            let stakes = self.stakes_cache.stakes();
            (
                stakes.stake_delegations().len(),
                stakes.vote_accounts().len(),
                stakes.staked_nodes().len(),
            )
        };
        self.capitalization
            .fetch_add(validator_rewards_paid, Relaxed);

        let active_stake = if let Some(stake_history_entry) =
            self.stakes_cache.stakes().history().get(prev_epoch)
        {
            stake_history_entry.effective
        } else {
            0
        };

        datapoint_warn!(
            "epoch_rewards",
            ("slot", self.slot, i64),
            ("epoch", prev_epoch, i64),
            ("validator_rate", validator_rate, f64),
            ("foundation_rate", foundation_rate, f64),
            ("epoch_duration_in_years", prev_epoch_duration_in_years, f64),
            ("validator_rewards", validator_rewards_paid, i64),
            ("active_stake", active_stake, i64),
            ("pre_capitalization", capitalization, i64),
            ("post_capitalization", self.capitalization(), i64),
            ("num_stake_accounts", num_stake_accounts as i64, i64),
            ("num_vote_accounts", num_vote_accounts as i64, i64),
            ("num_staked_nodes", num_staked_nodes as i64, i64)
        );
    }

    /// map stake delegations into resolved (pubkey, account) pairs
    ///  returns a map (has to be copied) of loaded
    ///   ( Vec<(staker info)> (voter account) ) keyed by voter pubkey
    ///
    /// Filters out invalid pairs
    fn load_vote_and_stake_accounts_with_thread_pool(
        &self,
        thread_pool: &ThreadPool,
        reward_calc_tracer: Option<impl Fn(&RewardCalculationEvent) + Send + Sync>,
    ) -> LoadVoteAndStakeAccountsResult {
        let stakes = self.stakes_cache.stakes();
        let cached_vote_accounts = stakes.vote_accounts();
        let vote_with_stake_delegations_map = DashMap::with_capacity(cached_vote_accounts.len());
        let invalid_stake_keys: DashMap<Pubkey, InvalidCacheEntryReason> = DashMap::new();
        let invalid_vote_keys: DashMap<Pubkey, InvalidCacheEntryReason> = DashMap::new();
        let invalid_cached_stake_accounts = AtomicUsize::default();
        let invalid_cached_vote_accounts = AtomicUsize::default();
        let invalid_cached_stake_accounts_rent_epoch = AtomicUsize::default();

        let stake_delegations: Vec<_> = stakes.stake_delegations().iter().collect();
        thread_pool.install(|| {
            stake_delegations
                .into_par_iter()
                .for_each(|(stake_pubkey, cached_stake_account)| {
                    let delegation = cached_stake_account.delegation();
                    let vote_pubkey = &delegation.voter_pubkey;
                    if invalid_vote_keys.contains_key(vote_pubkey) {
                        return;
                    }
                    let stake_account = match self.get_account_with_fixed_root(stake_pubkey) {
                        Some(stake_account) => stake_account,
                        None => {
                            invalid_stake_keys
                                .insert(*stake_pubkey, InvalidCacheEntryReason::Missing);
                            return;
                        }
                    };
                    let stake_account = match StakeAccount::<()>::try_from(stake_account) {
                        Ok(stake_account) => stake_account,
                        Err(stake_account::Error::InvalidOwner { .. }) => {
                            invalid_stake_keys
                                .insert(*stake_pubkey, InvalidCacheEntryReason::WrongOwner);
                            return;
                        }
                        Err(stake_account::Error::InstructionError(_)) => {
                            invalid_stake_keys
                                .insert(*stake_pubkey, InvalidCacheEntryReason::BadState);
                            return;
                        }
                        Err(stake_account::Error::InvalidDelegation(_)) => {
                            // This should not happen.
                            error!(
                                "Unexpected code path! StakeAccount<()> \
                                should not check if stake-state is a \
                                Delegation."
                            );
                            return;
                        }
                    };
                    if cached_stake_account != &stake_account {
                        invalid_cached_stake_accounts.fetch_add(1, Relaxed);
                        let mut cached_account = cached_stake_account.account().clone();
                        // We could have collected rent on the loaded account already in this new epoch (we could be at partition_index 12, for example).
                        // So, we may need to adjust the rent_epoch of the cached account. So, update rent_epoch and compare just the accounts.
                        ExpectedRentCollection::maybe_update_rent_epoch_on_load(
                            &mut cached_account,
                            &SlotInfoInEpoch::new_small(self.slot()),
                            &SlotInfoInEpoch::new_small(self.slot()),
                            self.epoch_schedule(),
                            self.rent_collector(),
                            stake_pubkey,
                            &self.rewrites_skipped_this_slot,
                        );
                        if &cached_account != stake_account.account() {
                            info!(
                                "cached stake account mismatch: {}: {:?}, {:?}",
                                stake_pubkey,
                                cached_account,
                                stake_account.account()
                            );
                        } else {
                            // track how many of 'invalid_cached_stake_accounts' were due to rent_epoch changes
                            // subtract these to find real invalid cached accounts
                            invalid_cached_stake_accounts_rent_epoch.fetch_add(1, Relaxed);
                        }
                    }
                    let stake_delegation = (*stake_pubkey, stake_account);
                    let mut vote_delegations = if let Some(vote_delegations) =
                        vote_with_stake_delegations_map.get_mut(vote_pubkey)
                    {
                        vote_delegations
                    } else {
                        let cached_vote_account = cached_vote_accounts.get(vote_pubkey);
                        let vote_account = match self.get_account_with_fixed_root(vote_pubkey) {
                            Some(vote_account) => {
                                match cached_vote_account {
                                    Some(cached_vote_account)
                                        if cached_vote_account == &vote_account => {}
                                    _ => {
                                        invalid_cached_vote_accounts.fetch_add(1, Relaxed);
                                    }
                                };
                                if vote_account.owner() != &solana_vote_program::id() {
                                    invalid_vote_keys
                                        .insert(*vote_pubkey, InvalidCacheEntryReason::WrongOwner);
                                    return;
                                }
                                vote_account
                            }
                            None => {
                                if cached_vote_account.is_some() {
                                    invalid_cached_vote_accounts.fetch_add(1, Relaxed);
                                }
                                invalid_vote_keys
                                    .insert(*vote_pubkey, InvalidCacheEntryReason::Missing);
                                return;
                            }
                        };

                        let vote_state = if let Ok(vote_state) =
                            StateMut::<VoteStateVersions>::state(&vote_account)
                        {
                            vote_state.convert_to_current()
                        } else {
                            invalid_vote_keys
                                .insert(*vote_pubkey, InvalidCacheEntryReason::BadState);
                            return;
                        };

                        vote_with_stake_delegations_map
                            .entry(*vote_pubkey)
                            .or_insert_with(|| VoteWithStakeDelegations {
                                vote_state: Arc::new(vote_state),
                                vote_account,
                                delegations: vec![],
                            })
                    };

                    if let Some(reward_calc_tracer) = reward_calc_tracer.as_ref() {
                        reward_calc_tracer(&RewardCalculationEvent::Staking(
                            stake_pubkey,
                            &InflationPointCalculationEvent::Delegation(
                                delegation,
                                solana_vote_program::id(),
                            ),
                        ));
                    }

                    vote_delegations.delegations.push(stake_delegation);
                });
        });
        invalid_cached_stake_accounts.fetch_add(invalid_stake_keys.len(), Relaxed);
        LoadVoteAndStakeAccountsResult {
            vote_with_stake_delegations_map,
            invalid_vote_keys,
            invalid_stake_keys,
            invalid_cached_vote_accounts: invalid_cached_vote_accounts.into_inner(),
            invalid_cached_stake_accounts: invalid_cached_stake_accounts.into_inner(),
            invalid_cached_stake_accounts_rent_epoch: invalid_cached_stake_accounts_rent_epoch
                .into_inner(),
            vote_accounts_cache_miss_count: 0,
        }
    }

    fn load_vote_and_stake_accounts<F>(
        &self,
        thread_pool: &ThreadPool,
        reward_calc_tracer: Option<F>,
    ) -> LoadVoteAndStakeAccountsResult
    where
        F: Fn(&RewardCalculationEvent) + Send + Sync,
    {
        let stakes = self.stakes_cache.stakes();
        let stake_delegations: Vec<_> = stakes.stake_delegations().iter().collect();
        // Obtain all unique voter pubkeys from stake delegations.
        fn merge(mut acc: HashSet<Pubkey>, other: HashSet<Pubkey>) -> HashSet<Pubkey> {
            if acc.len() < other.len() {
                return merge(other, acc);
            }
            acc.extend(other);
            acc
        }
        let voter_pubkeys = thread_pool.install(|| {
            stake_delegations
                .par_iter()
                .fold(
                    HashSet::default,
                    |mut voter_pubkeys, (_stake_pubkey, stake_account)| {
                        let delegation = stake_account.delegation();
                        voter_pubkeys.insert(delegation.voter_pubkey);
                        voter_pubkeys
                    },
                )
                .reduce(HashSet::default, merge)
        });
        // Obtain vote-accounts for unique voter pubkeys.
        let cached_vote_accounts = stakes.vote_accounts();
        let solana_vote_program: Pubkey = solana_vote_program::id();
        let vote_accounts_cache_miss_count = AtomicUsize::default();
        let get_vote_account = |vote_pubkey: &Pubkey| -> Option<VoteAccount> {
            if let Some(vote_account) = cached_vote_accounts.get(vote_pubkey) {
                return Some(vote_account.clone());
            }
            // If accounts-db contains a valid vote account, then it should
            // already have been cached in cached_vote_accounts; so the code
            // below is only for sanity check, and can be removed once
            // vote_accounts_cache_miss_count is shown to be always zero.
            let account = self.get_account_with_fixed_root(vote_pubkey)?;
            if account.owner() == &solana_vote_program
                && VoteState::deserialize(account.data()).is_ok()
            {
                vote_accounts_cache_miss_count.fetch_add(1, Relaxed);
            }
            VoteAccount::try_from(account).ok()
        };
        let invalid_vote_keys = DashMap::<Pubkey, InvalidCacheEntryReason>::new();
        let make_vote_delegations_entry = |vote_pubkey| {
            let vote_account = match get_vote_account(&vote_pubkey) {
                Some(vote_account) => vote_account,
                None => {
                    invalid_vote_keys.insert(vote_pubkey, InvalidCacheEntryReason::Missing);
                    return None;
                }
            };
            if vote_account.owner() != &solana_vote_program {
                invalid_vote_keys.insert(vote_pubkey, InvalidCacheEntryReason::WrongOwner);
                return None;
            }
            let vote_state = match vote_account.vote_state().deref() {
                Ok(vote_state) => vote_state.clone(),
                Err(_) => {
                    invalid_vote_keys.insert(vote_pubkey, InvalidCacheEntryReason::BadState);
                    return None;
                }
            };
            let vote_with_stake_delegations = VoteWithStakeDelegations {
                vote_state: Arc::new(vote_state),
                vote_account: AccountSharedData::from(vote_account),
                delegations: Vec::default(),
            };
            Some((vote_pubkey, vote_with_stake_delegations))
        };
        let vote_with_stake_delegations_map: DashMap<Pubkey, VoteWithStakeDelegations> =
            thread_pool.install(|| {
                voter_pubkeys
                    .into_par_iter()
                    .filter_map(make_vote_delegations_entry)
                    .collect()
            });
        // Join stake accounts with vote-accounts.
        let push_stake_delegation = |(stake_pubkey, stake_account): (&Pubkey, &StakeAccount<_>)| {
            let delegation = stake_account.delegation();
            let mut vote_delegations =
                match vote_with_stake_delegations_map.get_mut(&delegation.voter_pubkey) {
                    Some(vote_delegations) => vote_delegations,
                    None => return,
                };
            if let Some(reward_calc_tracer) = reward_calc_tracer.as_ref() {
                let delegation =
                    InflationPointCalculationEvent::Delegation(delegation, solana_vote_program);
                let event = RewardCalculationEvent::Staking(stake_pubkey, &delegation);
                reward_calc_tracer(&event);
            }
            let stake_account = StakeAccount::from(stake_account.clone());
            let stake_delegation = (*stake_pubkey, stake_account);
            vote_delegations.delegations.push(stake_delegation);
        };
        thread_pool.install(|| {
            stake_delegations
                .into_par_iter()
                .for_each(push_stake_delegation);
        });
        LoadVoteAndStakeAccountsResult {
            vote_with_stake_delegations_map,
            invalid_vote_keys,
            invalid_stake_keys: DashMap::default(),
            invalid_cached_vote_accounts: 0,
            invalid_cached_stake_accounts: 0,
            invalid_cached_stake_accounts_rent_epoch: 0,
            vote_accounts_cache_miss_count: vote_accounts_cache_miss_count.into_inner(),
        }
    }

    /// iterate over all stakes, redeem vote credits for each stake we can
    /// successfully load and parse, return the lamport value of one point
    fn pay_validator_rewards_with_thread_pool(
        &mut self,
        rewarded_epoch: Epoch,
        rewards: u64,
        reward_calc_tracer: Option<impl Fn(&RewardCalculationEvent) + Send + Sync>,
        credits_auto_rewind: bool,
        thread_pool: &ThreadPool,
        metrics: &mut RewardsMetrics,
        update_rewards_from_cached_accounts: bool,
    ) -> f64 {
        let stake_history = self.stakes_cache.stakes().history().clone();
        let vote_with_stake_delegations_map = {
            let mut m = Measure::start("load_vote_and_stake_accounts_us");
            let LoadVoteAndStakeAccountsResult {
                vote_with_stake_delegations_map,
                invalid_stake_keys,
                invalid_vote_keys,
                invalid_cached_vote_accounts,
                invalid_cached_stake_accounts,
                invalid_cached_stake_accounts_rent_epoch,
                vote_accounts_cache_miss_count,
            } = if update_rewards_from_cached_accounts {
                self.load_vote_and_stake_accounts(thread_pool, reward_calc_tracer.as_ref())
            } else {
                self.load_vote_and_stake_accounts_with_thread_pool(
                    thread_pool,
                    reward_calc_tracer.as_ref(),
                )
            };
            m.stop();
            metrics
                .load_vote_and_stake_accounts_us
                .fetch_add(m.as_us(), Relaxed);
            metrics.invalid_cached_vote_accounts += invalid_cached_vote_accounts;
            metrics.invalid_cached_stake_accounts += invalid_cached_stake_accounts;
            metrics.invalid_cached_stake_accounts_rent_epoch +=
                invalid_cached_stake_accounts_rent_epoch;
            metrics.vote_accounts_cache_miss_count += vote_accounts_cache_miss_count;
            self.stakes_cache.handle_invalid_keys(
                invalid_stake_keys,
                invalid_vote_keys,
                self.slot(),
            );
            vote_with_stake_delegations_map
        };

        let mut m = Measure::start("calculate_points");
        let points: u128 = thread_pool.install(|| {
            vote_with_stake_delegations_map
                .par_iter()
                .map(|entry| {
                    let VoteWithStakeDelegations {
                        vote_state,
                        delegations,
                        ..
                    } = entry.value();

                    delegations
                        .par_iter()
                        .map(|(_stake_pubkey, stake_account)| {
                            stake_state::calculate_points(
                                stake_account.stake_state(),
                                vote_state,
                                Some(&stake_history),
                            )
                            .unwrap_or(0)
                        })
                        .sum::<u128>()
                })
                .sum()
        });
        m.stop();
        metrics.calculate_points_us.fetch_add(m.as_us(), Relaxed);

        if points == 0 {
            return 0.0;
        }

        // pay according to point value
        let point_value = PointValue { rewards, points };
        let vote_account_rewards: DashMap<Pubkey, (AccountSharedData, u8, u64, bool)> =
            DashMap::with_capacity(vote_with_stake_delegations_map.len());
        let stake_delegation_iterator = vote_with_stake_delegations_map.into_par_iter().flat_map(
            |(
                vote_pubkey,
                VoteWithStakeDelegations {
                    vote_state,
                    vote_account,
                    delegations,
                },
            )| {
                vote_account_rewards
                    .insert(vote_pubkey, (vote_account, vote_state.commission, 0, false));
                delegations
                    .into_par_iter()
                    .map(move |delegation| (vote_pubkey, Arc::clone(&vote_state), delegation))
            },
        );

        let mut m = Measure::start("redeem_rewards");
        let stake_rewards: Vec<StakeReward> = thread_pool.install(|| {
            stake_delegation_iterator
                .filter_map(|(vote_pubkey, vote_state, (stake_pubkey, stake_account))| {
                    // curry closure to add the contextual stake_pubkey
                    let reward_calc_tracer = reward_calc_tracer.as_ref().map(|outer| {
                        // inner
                        move |inner_event: &_| {
                            outer(&RewardCalculationEvent::Staking(&stake_pubkey, inner_event))
                        }
                    });
                    let (mut stake_account, stake_state) =
                        <(AccountSharedData, StakeState)>::from(stake_account);
                    let redeemed = stake_state::redeem_rewards(
                        rewarded_epoch,
                        stake_state,
                        &mut stake_account,
                        &vote_state,
                        &point_value,
                        Some(&stake_history),
                        reward_calc_tracer.as_ref(),
                        credits_auto_rewind,
                    );
                    if let Ok((stakers_reward, voters_reward)) = redeemed {
                        // track voter rewards
                        if let Some((
                            _vote_account,
                            _commission,
                            vote_rewards_sum,
                            vote_needs_store,
                        )) = vote_account_rewards.get_mut(&vote_pubkey).as_deref_mut()
                        {
                            *vote_needs_store = true;
                            *vote_rewards_sum = vote_rewards_sum.saturating_add(voters_reward);
                        }

                        let post_balance = stake_account.lamports();
                        return Some(StakeReward {
                            stake_pubkey,
                            stake_reward_info: RewardInfo {
                                reward_type: RewardType::Staking,
                                lamports: i64::try_from(stakers_reward).unwrap(),
                                post_balance,
                                commission: Some(vote_state.commission),
                            },
                            stake_account,
                        });
                    } else {
                        debug!(
                            "stake_state::redeem_rewards() failed for {}: {:?}",
                            stake_pubkey, redeemed
                        );
                    }
                    None
                })
                .collect()
        });
        m.stop();
        metrics.redeem_rewards_us += m.as_us();

        // store stake account even if stakers_reward is 0
        // because credits observed has changed
        let mut m = Measure::start("store_stake_account");
        self.store_accounts((self.slot(), &stake_rewards[..]));
        m.stop();
        metrics
            .store_stake_accounts_us
            .fetch_add(m.as_us(), Relaxed);

        let mut m = Measure::start("store_vote_accounts");
        let mut vote_rewards = vote_account_rewards
            .into_iter()
            .filter_map(
                |(vote_pubkey, (mut vote_account, commission, vote_rewards, vote_needs_store))| {
                    if let Err(err) = vote_account.checked_add_lamports(vote_rewards) {
                        debug!("reward redemption failed for {}: {:?}", vote_pubkey, err);
                        return None;
                    }

                    if vote_needs_store {
                        self.store_account(&vote_pubkey, &vote_account);
                    }

                    if vote_rewards > 0 {
                        Some((
                            vote_pubkey,
                            RewardInfo {
                                reward_type: RewardType::Voting,
                                lamports: vote_rewards as i64,
                                post_balance: vote_account.lamports(),
                                commission: Some(commission),
                            },
                        ))
                    } else {
                        None
                    }
                },
            )
            .collect::<Vec<_>>();

        m.stop();
        metrics.store_vote_accounts_us.fetch_add(m.as_us(), Relaxed);

        let additional_reserve = stake_rewards.len() + vote_rewards.len();
        {
            let mut rewards = self.rewards.write().unwrap();
            rewards.reserve(additional_reserve);
            rewards.append(&mut vote_rewards);
            stake_rewards
                .into_iter()
                .filter(|x| x.get_stake_reward() > 0)
                .for_each(|x| rewards.push((x.stake_pubkey, x.stake_reward_info)));
        }

        point_value.rewards as f64 / point_value.points as f64
    }

    fn update_recent_blockhashes_locked(&self, locked_blockhash_queue: &BlockhashQueue) {
        #[allow(deprecated)]
        self.update_sysvar_account(&sysvar::recent_blockhashes::id(), |account| {
            let recent_blockhash_iter = locked_blockhash_queue.get_recent_blockhashes();
            recent_blockhashes_account::create_account_with_data_and_fields(
                recent_blockhash_iter,
                self.inherit_specially_retained_account_fields(account),
            )
        });
    }

    pub fn update_recent_blockhashes(&self) {
        let blockhash_queue = self.blockhash_queue.read().unwrap();
        self.update_recent_blockhashes_locked(&blockhash_queue);
    }

    fn get_timestamp_estimate(
        &self,
        max_allowable_drift: MaxAllowableDrift,
        epoch_start_timestamp: Option<(Slot, UnixTimestamp)>,
    ) -> Option<UnixTimestamp> {
        let mut get_timestamp_estimate_time = Measure::start("get_timestamp_estimate");
        let slots_per_epoch = self.epoch_schedule().slots_per_epoch;
        let vote_accounts = self.vote_accounts();
        let recent_timestamps = vote_accounts.iter().filter_map(|(pubkey, (_, account))| {
            let vote_state = account.vote_state();
            let vote_state = vote_state.as_ref().ok()?;
            let slot_delta = self.slot().checked_sub(vote_state.last_timestamp.slot)?;
            (slot_delta <= slots_per_epoch).then(|| {
                (
                    *pubkey,
                    (
                        vote_state.last_timestamp.slot,
                        vote_state.last_timestamp.timestamp,
                    ),
                )
            })
        });
        let slot_duration = Duration::from_nanos(self.ns_per_slot as u64);
        let epoch = self.epoch_schedule().get_epoch(self.slot());
        let stakes = self.epoch_vote_accounts(epoch)?;
        let stake_weighted_timestamp = calculate_stake_weighted_timestamp(
            recent_timestamps,
            stakes,
            self.slot(),
            slot_duration,
            epoch_start_timestamp,
            max_allowable_drift,
            self.feature_set
                .is_active(&feature_set::warp_timestamp_again::id()),
        );
        get_timestamp_estimate_time.stop();
        datapoint_info!(
            "bank-timestamp",
            (
                "get_timestamp_estimate_us",
                get_timestamp_estimate_time.as_us(),
                i64
            ),
        );
        stake_weighted_timestamp
    }

    // Distribute collected transaction fees for this slot to collector_id (= current leader).
    //
    // Each validator is incentivized to process more transactions to earn more transaction fees.
    // Transaction fees are rewarded for the computing resource utilization cost, directly
    // proportional to their actual processing power.
    //
    // collector_id is rotated according to stake-weighted leader schedule. So the opportunity of
    // earning transaction fees are fairly distributed by stake. And missing the opportunity
    // (not producing a block as a leader) earns nothing. So, being online is incentivized as a
    // form of transaction fees as well.
    //
    // On the other hand, rent fees are distributed under slightly different philosophy, while
    // still being stake-weighted.
    // Ref: distribute_rent_to_validators
    fn collect_fees(&self) {
        let collector_fees = self.collector_fees.load(Relaxed) as u64;

        if collector_fees != 0 {
            let (deposit, mut burn) = self.fee_rate_governor.burn(collector_fees);
            // burn a portion of fees
            debug!(
                "distributed fee: {} (rounded from: {}, burned: {})",
                deposit, collector_fees, burn
            );

            match self.deposit(&self.collector_id, deposit) {
                Ok(post_balance) => {
                    if deposit != 0 {
                        self.rewards.write().unwrap().push((
                            self.collector_id,
                            RewardInfo {
                                reward_type: RewardType::Fee,
                                lamports: deposit as i64,
                                post_balance,
                                commission: None,
                            },
                        ));
                    }
                }
                Err(_) => {
                    error!(
                        "Burning {} fee instead of crediting {}",
                        deposit, self.collector_id
                    );
                    inc_new_counter_error!("bank-burned_fee_lamports", deposit as usize);
                    burn += deposit;
                }
            }
            self.capitalization.fetch_sub(burn, Relaxed);
        }
    }

    pub fn rehash(&self) {
        let mut hash = self.hash.write().unwrap();
        let new = self.hash_internal_state();
        if new != *hash {
            warn!("Updating bank hash to {}", new);
            *hash = new;
        }
    }

    pub fn freeze(&self) {
        // This lock prevents any new commits from BankingStage
        // `process_and_record_transactions_locked()` from coming
        // in after the last tick is observed. This is because in
        // BankingStage, any transaction successfully recorded in
        // `record_transactions()` is recorded after this `hash` lock
        // is grabbed. At the time of the successful record,
        // this means the PoH has not yet reached the last tick,
        // so this means freeze() hasn't been called yet. And because
        // BankingStage doesn't release this hash lock until both
        // record and commit are finished, those transactions will be
        // committed before this write lock can be obtained here.
        let mut hash = self.hash.write().unwrap();
        if *hash == Hash::default() {
            // finish up any deferred changes to account state
            self.collect_rent_eagerly(false);
            self.collect_fees();
            self.distribute_rent();
            self.update_slot_history();
            self.run_incinerator();

            // freeze is a one-way trip, idempotent
            self.freeze_started.store(true, Relaxed);
            *hash = self.hash_internal_state();
            self.rc.accounts.accounts_db.mark_slot_frozen(self.slot());
        }
    }

    // dangerous; don't use this; this is only needed for ledger-tool's special command
    pub fn unfreeze_for_ledger_tool(&self) {
        self.freeze_started.store(false, Relaxed);
    }

    pub fn epoch_schedule(&self) -> &EpochSchedule {
        &self.epoch_schedule
    }

    /// squash the parent's state up into this Bank,
    ///   this Bank becomes a root
    pub fn squash(&self) -> SquashTiming {
        self.freeze();

        //this bank and all its parents are now on the rooted path
        let mut roots = vec![self.slot()];
        roots.append(&mut self.parents().iter().map(|p| p.slot()).collect());

        let mut total_index_us = 0;
        let mut total_cache_us = 0;
        let mut total_store_us = 0;

        let mut squash_accounts_time = Measure::start("squash_accounts_time");
        for slot in roots.iter().rev() {
            // root forks cannot be purged
            let add_root_timing = self.rc.accounts.add_root(*slot);
            total_index_us += add_root_timing.index_us;
            total_cache_us += add_root_timing.cache_us;
            total_store_us += add_root_timing.store_us;
        }
        squash_accounts_time.stop();

        *self.rc.parent.write().unwrap() = None;

        let mut squash_cache_time = Measure::start("squash_cache_time");
        roots
            .iter()
            .for_each(|slot| self.status_cache.write().unwrap().add_root(*slot));
        squash_cache_time.stop();

        SquashTiming {
            squash_accounts_ms: squash_accounts_time.as_ms(),
            squash_accounts_index_ms: total_index_us / 1000,
            squash_accounts_cache_ms: total_cache_us / 1000,
            squash_accounts_store_ms: total_store_us / 1000,

            squash_cache_ms: squash_cache_time.as_ms(),
        }
    }

    /// Return the more recent checkpoint of this bank instance.
    pub fn parent(&self) -> Option<Arc<Bank>> {
        self.rc.parent.read().unwrap().clone()
    }

    pub fn parent_slot(&self) -> Slot {
        self.parent_slot
    }

    pub fn parent_hash(&self) -> Hash {
        self.parent_hash
    }

    fn process_genesis_config(&mut self, genesis_config: &GenesisConfig) {
        // Bootstrap validator collects fees until `new_from_parent` is called.
        self.fee_rate_governor = genesis_config.fee_rate_governor.clone();
        self.fee_calculator = self.fee_rate_governor.create_fee_calculator();

        for (pubkey, account) in genesis_config.accounts.iter() {
            assert!(
                self.get_account(pubkey).is_none(),
                "{} repeated in genesis config",
                pubkey
            );
            self.store_account(pubkey, account);
            self.capitalization.fetch_add(account.lamports(), Relaxed);
            self.accounts_data_size_initial += account.data().len() as u64;
        }
        // updating sysvars (the fees sysvar in this case) now depends on feature activations in
        // genesis_config.accounts above
        self.update_fees();

        for (pubkey, account) in genesis_config.rewards_pools.iter() {
            assert!(
                self.get_account(pubkey).is_none(),
                "{} repeated in genesis config",
                pubkey
            );
            self.store_account(pubkey, account);
            self.accounts_data_size_initial += account.data().len() as u64;
        }

        // highest staked node is the first collector
        self.collector_id = self
            .stakes_cache
            .stakes()
            .highest_staked_node()
            .unwrap_or_default();

        self.blockhash_queue.write().unwrap().genesis_hash(
            &genesis_config.hash(),
            self.fee_rate_governor.lamports_per_signature,
        );

        self.hashes_per_tick = genesis_config.hashes_per_tick();
        self.ticks_per_slot = genesis_config.ticks_per_slot();
        self.ns_per_slot = genesis_config.ns_per_slot();
        self.genesis_creation_time = genesis_config.creation_time;
        self.max_tick_height = (self.slot + 1) * self.ticks_per_slot;
        self.slots_per_year = genesis_config.slots_per_year();

        self.epoch_schedule = genesis_config.epoch_schedule;

        self.inflation = Arc::new(RwLock::new(genesis_config.inflation));

        self.rent_collector = RentCollector::new(
            self.epoch,
            *self.epoch_schedule(),
            self.slots_per_year,
            genesis_config.rent,
        );

        // Add additional builtin programs specified in the genesis config
        for (name, program_id) in &genesis_config.native_instruction_processors {
            self.add_builtin_account(name, program_id, false);
        }
    }

    fn burn_and_purge_account(&self, program_id: &Pubkey, mut account: AccountSharedData) {
        let old_data_size = account.data().len();
        self.capitalization.fetch_sub(account.lamports(), Relaxed);
        // Both resetting account balance to 0 and zeroing the account data
        // is needed to really purge from AccountsDb and flush the Stakes cache
        account.set_lamports(0);
        account.data_as_mut_slice().fill(0);
        self.store_account(program_id, &account);
        self.calculate_and_update_accounts_data_size_delta_off_chain(old_data_size, 0);
    }

    // NOTE: must hold idempotent for the same set of arguments
    /// Add a builtin program account
    pub fn add_builtin_account(&self, name: &str, program_id: &Pubkey, must_replace: bool) {
        let existing_genuine_program =
            self.get_account_with_fixed_root(program_id)
                .and_then(|account| {
                    // it's very unlikely to be squatted at program_id as non-system account because of burden to
                    // find victim's pubkey/hash. So, when account.owner is indeed native_loader's, it's
                    // safe to assume it's a genuine program.
                    if native_loader::check_id(account.owner()) {
                        Some(account)
                    } else {
                        // malicious account is pre-occupying at program_id
                        self.burn_and_purge_account(program_id, account);
                        None
                    }
                });

        if must_replace {
            // updating builtin program
            match &existing_genuine_program {
                None => panic!(
                    "There is no account to replace with builtin program ({}, {}).",
                    name, program_id
                ),
                Some(account) => {
                    if *name == String::from_utf8_lossy(account.data()) {
                        // The existing account is well formed
                        return;
                    }
                }
            }
        } else {
            // introducing builtin program
            if existing_genuine_program.is_some() {
                // The existing account is sufficient
                return;
            }
        }

        assert!(
            !self.freeze_started(),
            "Can't change frozen bank by adding not-existing new builtin program ({}, {}). \
            Maybe, inconsistent program activation is detected on snapshot restore?",
            name,
            program_id
        );

        // Add a bogus executable builtin account, which will be loaded and ignored.
        let account = native_loader::create_loadable_account_with_fields(
            name,
            self.inherit_specially_retained_account_fields(&existing_genuine_program),
        );
        self.store_account_and_update_capitalization(program_id, &account);
    }

    /// Add a precompiled program account
    pub fn add_precompiled_account(&self, program_id: &Pubkey) {
        self.add_precompiled_account_with_owner(program_id, native_loader::id())
    }

    // Used by tests to simulate clusters with precompiles that aren't owned by the native loader
    fn add_precompiled_account_with_owner(&self, program_id: &Pubkey, owner: Pubkey) {
        if let Some(account) = self.get_account_with_fixed_root(program_id) {
            if account.executable() {
                // The account is already executable, that's all we need
                return;
            } else {
                // malicious account is pre-occupying at program_id
                self.burn_and_purge_account(program_id, account);
            }
        };

        assert!(
            !self.freeze_started(),
            "Can't change frozen bank by adding not-existing new precompiled program ({}). \
                Maybe, inconsistent program activation is detected on snapshot restore?",
            program_id
        );

        // Add a bogus executable account, which will be loaded and ignored.
        let (lamports, rent_epoch) = self.inherit_specially_retained_account_fields(&None);
        let account = AccountSharedData::from(Account {
            lamports,
            owner,
            data: vec![],
            executable: true,
            rent_epoch,
        });
        self.store_account_and_update_capitalization(program_id, &account);
    }

    pub fn set_rent_burn_percentage(&mut self, burn_percent: u8) {
        self.rent_collector.rent.burn_percent = burn_percent;
    }

    pub fn set_hashes_per_tick(&mut self, hashes_per_tick: Option<u64>) {
        self.hashes_per_tick = hashes_per_tick;
    }

    /// Return the last block hash registered.
    pub fn last_blockhash(&self) -> Hash {
        self.blockhash_queue.read().unwrap().last_hash()
    }

    pub fn last_blockhash_and_lamports_per_signature(&self) -> (Hash, u64) {
        let blockhash_queue = self.blockhash_queue.read().unwrap();
        let last_hash = blockhash_queue.last_hash();
        let last_lamports_per_signature = blockhash_queue
            .get_lamports_per_signature(&last_hash)
            .unwrap(); // safe so long as the BlockhashQueue is consistent
        (last_hash, last_lamports_per_signature)
    }

    pub fn is_blockhash_valid(&self, hash: &Hash) -> bool {
        let blockhash_queue = self.blockhash_queue.read().unwrap();
        blockhash_queue.is_hash_valid(hash)
    }

    pub fn get_minimum_balance_for_rent_exemption(&self, data_len: usize) -> u64 {
        self.rent_collector.rent.minimum_balance(data_len).max(1)
    }

    pub fn get_lamports_per_signature(&self) -> u64 {
        self.fee_rate_governor.lamports_per_signature
    }

    pub fn get_lamports_per_signature_for_blockhash(&self, hash: &Hash) -> Option<u64> {
        let blockhash_queue = self.blockhash_queue.read().unwrap();
        blockhash_queue.get_lamports_per_signature(hash)
    }

    #[deprecated(since = "1.9.0", note = "Please use `get_fee_for_message` instead")]
    pub fn get_fee_rate_governor(&self) -> &FeeRateGovernor {
        &self.fee_rate_governor
    }

    pub fn get_fee_for_message(&self, message: &SanitizedMessage) -> Option<u64> {
        let lamports_per_signature = {
            let blockhash_queue = self.blockhash_queue.read().unwrap();
            blockhash_queue.get_lamports_per_signature(message.recent_blockhash())
        }
        .or_else(|| {
            self.check_message_for_nonce(message)
                .and_then(|(address, account)| {
                    NoncePartial::new(address, account).lamports_per_signature()
                })
        })?;
        Some(Self::calculate_fee(
            message,
            lamports_per_signature,
            &self.fee_structure,
            self.feature_set
                .is_active(&add_set_compute_unit_price_ix::id()),
            self.feature_set
                .is_active(&use_default_units_in_fee_calculation::id()),
        ))
    }

    pub fn get_startup_verification_complete(&self) -> &Arc<AtomicBool> {
        &self
            .rc
            .accounts
            .accounts_db
            .verify_accounts_hash_in_bg
            .verified
    }

    pub fn is_startup_verification_complete(&self) -> bool {
        self.rc
            .accounts
            .accounts_db
            .verify_accounts_hash_in_bg
            .check_complete()
    }

    /// This can occur because it completed in the background
    /// or if the verification was run in the foreground.
    pub fn set_startup_verification_complete(&self) {
        self.rc
            .accounts
            .accounts_db
            .verify_accounts_hash_in_bg
            .verification_complete()
    }

    pub fn get_fee_for_message_with_lamports_per_signature(
        &self,
        message: &SanitizedMessage,
        lamports_per_signature: u64,
    ) -> u64 {
        Self::calculate_fee(
            message,
            lamports_per_signature,
            &self.fee_structure,
            self.feature_set
                .is_active(&add_set_compute_unit_price_ix::id()),
            self.feature_set
                .is_active(&use_default_units_in_fee_calculation::id()),
        )
    }

    #[deprecated(
        since = "1.6.11",
        note = "Please use `get_blockhash_last_valid_block_height`"
    )]
    pub fn get_blockhash_last_valid_slot(&self, blockhash: &Hash) -> Option<Slot> {
        let blockhash_queue = self.blockhash_queue.read().unwrap();
        // This calculation will need to be updated to consider epoch boundaries if BlockhashQueue
        // length is made variable by epoch
        blockhash_queue
            .get_hash_age(blockhash)
            .map(|age| self.slot + blockhash_queue.get_max_age() as u64 - age)
    }

    pub fn get_blockhash_last_valid_block_height(&self, blockhash: &Hash) -> Option<Slot> {
        let blockhash_queue = self.blockhash_queue.read().unwrap();
        // This calculation will need to be updated to consider epoch boundaries if BlockhashQueue
        // length is made variable by epoch
        blockhash_queue
            .get_hash_age(blockhash)
            .map(|age| self.block_height + blockhash_queue.get_max_age() as u64 - age)
    }

    pub fn confirmed_last_blockhash(&self) -> Hash {
        const NUM_BLOCKHASH_CONFIRMATIONS: usize = 3;

        let parents = self.parents();
        if parents.is_empty() {
            self.last_blockhash()
        } else {
            let index = NUM_BLOCKHASH_CONFIRMATIONS.min(parents.len() - 1);
            parents[index].last_blockhash()
        }
    }

    /// Forget all signatures. Useful for benchmarking.
    pub fn clear_signatures(&self) {
        self.status_cache.write().unwrap().clear();
    }

    pub fn clear_slot_signatures(&self, slot: Slot) {
        self.status_cache.write().unwrap().clear_slot_entries(slot);
    }

    fn update_transaction_statuses(
        &self,
        sanitized_txs: &[SanitizedTransaction],
        execution_results: &[TransactionExecutionResult],
    ) {
        let mut status_cache = self.status_cache.write().unwrap();
        assert_eq!(sanitized_txs.len(), execution_results.len());
        for (tx, execution_result) in sanitized_txs.iter().zip(execution_results) {
            if let Some(details) = execution_result.details() {
                // Add the message hash to the status cache to ensure that this message
                // won't be processed again with a different signature.
                status_cache.insert(
                    tx.message().recent_blockhash(),
                    tx.message_hash(),
                    self.slot(),
                    details.status.clone(),
                );
                // Add the transaction signature to the status cache so that transaction status
                // can be queried by transaction signature over RPC. In the future, this should
                // only be added for API nodes because voting validators don't need to do this.
                status_cache.insert(
                    tx.message().recent_blockhash(),
                    tx.signature(),
                    self.slot(),
                    details.status.clone(),
                );
            }
        }
    }

    /// Register a new recent blockhash in the bank's recent blockhash queue. Called when a bank
    /// reaches its max tick height. Can be called by tests to get new blockhashes for transaction
    /// processing without advancing to a new bank slot.
    pub fn register_recent_blockhash(&self, blockhash: &Hash) {
        // Only acquire the write lock for the blockhash queue on block boundaries because
        // readers can starve this write lock acquisition and ticks would be slowed down too
        // much if the write lock is acquired for each tick.
        let mut w_blockhash_queue = self.blockhash_queue.write().unwrap();
        w_blockhash_queue.register_hash(blockhash, self.fee_rate_governor.lamports_per_signature);
        self.update_recent_blockhashes_locked(&w_blockhash_queue);
    }

    /// Tell the bank which Entry IDs exist on the ledger. This function assumes subsequent calls
    /// correspond to later entries, and will boot the oldest ones once its internal cache is full.
    /// Once boot, the bank will reject transactions using that `hash`.
    ///
    /// This is NOT thread safe because if tick height is updated by two different threads, the
    /// block boundary condition could be missed.
    pub fn register_tick(&self, hash: &Hash) {
        assert!(
            !self.freeze_started(),
            "register_tick() working on a bank that is already frozen or is undergoing freezing!"
        );

        inc_new_counter_debug!("bank-register_tick-registered", 1);
        if self.is_block_boundary(self.tick_height.load(Relaxed) + 1) {
            self.register_recent_blockhash(hash);
        }

        // ReplayStage will start computing the accounts delta hash when it
        // detects the tick height has reached the boundary, so the system
        // needs to guarantee all account updates for the slot have been
        // committed before this tick height is incremented (like the blockhash
        // sysvar above)
        self.tick_height.fetch_add(1, Relaxed);
    }

    pub fn is_complete(&self) -> bool {
        self.tick_height() == self.max_tick_height()
    }

    pub fn is_block_boundary(&self, tick_height: u64) -> bool {
        if self
            .feature_set
            .is_active(&feature_set::fix_recent_blockhashes::id())
        {
            tick_height == self.max_tick_height
        } else {
            tick_height % self.ticks_per_slot == 0
        }
    }

    /// Prepare a transaction batch from a list of legacy transactions. Used for tests only.
    pub fn prepare_batch_for_tests(&self, txs: Vec<Transaction>) -> TransactionBatch {
        let sanitized_txs = txs
            .into_iter()
            .map(SanitizedTransaction::from_transaction_for_tests)
            .collect::<Vec<_>>();
        let lock_results = self.rc.accounts.lock_accounts(sanitized_txs.iter());
        TransactionBatch::new(lock_results, self, Cow::Owned(sanitized_txs))
    }

    /// Prepare a transaction batch from a list of versioned transactions from
    /// an entry. Used for tests only.
    pub fn prepare_entry_batch(&self, txs: Vec<VersionedTransaction>) -> Result<TransactionBatch> {
        let sanitized_txs = txs
            .into_iter()
            .map(|tx| {
                SanitizedTransaction::try_create(
                    tx,
                    MessageHash::Compute,
                    None,
                    self,
                    self.feature_set
                        .is_active(&feature_set::require_static_program_ids_in_transaction::ID),
                )
            })
            .collect::<Result<Vec<_>>>()?;
        let lock_results = self.rc.accounts.lock_accounts(sanitized_txs.iter());
        Ok(TransactionBatch::new(
            lock_results,
            self,
            Cow::Owned(sanitized_txs),
        ))
    }

    /// Prepare a locked transaction batch from a list of sanitized transactions.
    pub fn prepare_sanitized_batch<'a, 'b>(
        &'a self,
        txs: &'b [SanitizedTransaction],
    ) -> TransactionBatch<'a, 'b> {
        let lock_results = self.rc.accounts.lock_accounts(txs.iter());
        TransactionBatch::new(lock_results, self, Cow::Borrowed(txs))
    }

    /// Prepare a locked transaction batch from a list of sanitized transactions, and their cost
    /// limited packing status
    pub fn prepare_sanitized_batch_with_results<'a, 'b>(
        &'a self,
        transactions: &'b [SanitizedTransaction],
        transaction_results: impl Iterator<Item = &'b Result<()>>,
    ) -> TransactionBatch<'a, 'b> {
        // this lock_results could be: Ok, AccountInUse, WouldExceedBlockMaxLimit or WouldExceedAccountMaxLimit
        let lock_results = self
            .rc
            .accounts
            .lock_accounts_with_results(transactions.iter(), transaction_results);
        TransactionBatch::new(lock_results, self, Cow::Borrowed(transactions))
    }

    /// Prepare a transaction batch without locking accounts for transaction simulation.
    pub(crate) fn prepare_simulation_batch<'a>(
        &'a self,
        transaction: SanitizedTransaction,
    ) -> TransactionBatch<'a, '_> {
        let lock_result = transaction.get_account_locks().map(|_| ());
        let mut batch =
            TransactionBatch::new(vec![lock_result], self, Cow::Owned(vec![transaction]));
        batch.set_needs_unlock(false);
        batch
    }

    /// Run transactions against a frozen bank without committing the results
    pub fn simulate_transaction(
        &self,
        transaction: SanitizedTransaction,
    ) -> TransactionSimulationResult {
        assert!(self.is_frozen(), "simulation bank must be frozen");

        self.simulate_transaction_unchecked(transaction)
    }

    /// Run transactions against a bank without committing the results; does not check if the bank
    /// is frozen, enabling use in single-Bank test frameworks
    pub fn simulate_transaction_unchecked(
        &self,
        transaction: SanitizedTransaction,
    ) -> TransactionSimulationResult {
        let account_keys = transaction.message().account_keys();
        let number_of_accounts = account_keys.len();
        let account_overrides = self.get_account_overrides_for_simulation(&account_keys);
        let batch = self.prepare_simulation_batch(transaction);
        let mut timings = ExecuteTimings::default();

        let LoadAndExecuteTransactionsOutput {
            loaded_transactions,
            mut execution_results,
            ..
        } = self.load_and_execute_transactions(
            &batch,
            // After simulation, transactions will need to be forwarded to the leader
            // for processing. During forwarding, the transaction could expire if the
            // delay is not accounted for.
            MAX_PROCESSING_AGE - MAX_TRANSACTION_FORWARDING_DELAY,
            false,
            true,
            true,
            &mut timings,
            Some(&account_overrides),
            None,
        );

        let post_simulation_accounts = loaded_transactions
            .into_iter()
            .next()
            .unwrap()
            .0
            .ok()
            .map(|loaded_transaction| {
                loaded_transaction
                    .accounts
                    .into_iter()
                    .take(number_of_accounts)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let units_consumed = timings
            .details
            .per_program_timings
            .iter()
            .fold(0, |acc: u64, (_, program_timing)| {
                acc.saturating_add(program_timing.accumulated_units)
            });

        debug!("simulate_transaction: {:?}", timings);

        let execution_result = execution_results.pop().unwrap();
        let flattened_result = execution_result.flattened_result();
        let (logs, return_data) = match execution_result {
            TransactionExecutionResult::Executed { details, .. } => {
                (details.log_messages, details.return_data)
            }
            TransactionExecutionResult::NotExecuted(_) => (None, None),
        };
        let logs = logs.unwrap_or_default();

        TransactionSimulationResult {
            result: flattened_result,
            logs,
            post_simulation_accounts,
            units_consumed,
            return_data,
        }
    }

    fn get_account_overrides_for_simulation(&self, account_keys: &AccountKeys) -> AccountOverrides {
        let mut account_overrides = AccountOverrides::default();
        let slot_history_id = sysvar::slot_history::id();
        if account_keys.iter().any(|pubkey| *pubkey == slot_history_id) {
            let current_account = self.get_account_with_fixed_root(&slot_history_id);
            let slot_history = current_account
                .as_ref()
                .map(|account| from_account::<SlotHistory, _>(account).unwrap())
                .unwrap_or_default();
            if slot_history.check(self.slot()) == Check::Found {
                let ancestors = Ancestors::from(self.proper_ancestors().collect::<Vec<_>>());
                if let Some((account, _)) =
                    self.load_slow_with_fixed_root(&ancestors, &slot_history_id)
                {
                    account_overrides.set_slot_history(Some(account));
                }
            }
        }
        account_overrides
    }

    pub fn unlock_accounts(&self, batch: &mut TransactionBatch) {
        if batch.needs_unlock() {
            batch.set_needs_unlock(false);
            self.rc
                .accounts
                .unlock_accounts(batch.sanitized_transactions().iter(), batch.lock_results())
        }
    }

    pub fn remove_unrooted_slots(&self, slots: &[(Slot, BankId)]) {
        self.rc.accounts.accounts_db.remove_unrooted_slots(slots)
    }

    pub fn set_shrink_paths(&self, paths: Vec<PathBuf>) {
        self.rc.accounts.accounts_db.set_shrink_paths(paths);
    }

    fn check_age<'a>(
        &self,
        txs: impl Iterator<Item = &'a SanitizedTransaction>,
        lock_results: &[Result<()>],
        max_age: usize,
        error_counters: &mut TransactionErrorMetrics,
    ) -> Vec<TransactionCheckResult> {
        let hash_queue = self.blockhash_queue.read().unwrap();
        let last_blockhash = hash_queue.last_hash();
        let next_durable_nonce = DurableNonce::from_blockhash(&last_blockhash);

        txs.zip(lock_results)
            .map(|(tx, lock_res)| match lock_res {
                Ok(()) => {
                    let recent_blockhash = tx.message().recent_blockhash();
                    if hash_queue.is_hash_valid_for_age(recent_blockhash, max_age) {
                        (Ok(()), None)
                    } else if let Some((address, account)) =
                        self.check_transaction_for_nonce(tx, &next_durable_nonce)
                    {
                        (Ok(()), Some(NoncePartial::new(address, account)))
                    } else {
                        error_counters.blockhash_not_found += 1;
                        (Err(TransactionError::BlockhashNotFound), None)
                    }
                }
                Err(e) => (Err(e.clone()), None),
            })
            .collect()
    }

    fn is_transaction_already_processed(
        &self,
        sanitized_tx: &SanitizedTransaction,
        status_cache: &BankStatusCache,
    ) -> bool {
        let key = sanitized_tx.message_hash();
        let transaction_blockhash = sanitized_tx.message().recent_blockhash();
        status_cache
            .get_status(key, transaction_blockhash, &self.ancestors)
            .is_some()
    }

    fn check_status_cache(
        &self,
        sanitized_txs: &[SanitizedTransaction],
        lock_results: Vec<TransactionCheckResult>,
        error_counters: &mut TransactionErrorMetrics,
    ) -> Vec<TransactionCheckResult> {
        let rcache = self.status_cache.read().unwrap();
        sanitized_txs
            .iter()
            .zip(lock_results)
            .map(|(sanitized_tx, (lock_result, nonce))| {
                if lock_result.is_ok()
                    && self.is_transaction_already_processed(sanitized_tx, &rcache)
                {
                    error_counters.already_processed += 1;
                    return (Err(TransactionError::AlreadyProcessed), None);
                }

                (lock_result, nonce)
            })
            .collect()
    }

    pub fn get_hash_age(&self, hash: &Hash) -> Option<u64> {
        self.blockhash_queue.read().unwrap().get_hash_age(hash)
    }

    pub fn is_hash_valid_for_age(&self, hash: &Hash, max_age: usize) -> bool {
        self.blockhash_queue
            .read()
            .unwrap()
            .is_hash_valid_for_age(hash, max_age)
    }

    fn check_message_for_nonce(&self, message: &SanitizedMessage) -> Option<TransactionAccount> {
        let nonce_address = message.get_durable_nonce()?;
        let nonce_account = self.get_account_with_fixed_root(nonce_address)?;
        let nonce_data =
            nonce_account::verify_nonce_account(&nonce_account, message.recent_blockhash())?;

        let nonce_is_authorized = message
            .get_ix_signers(NONCED_TX_MARKER_IX_INDEX as usize)
            .any(|signer| signer == &nonce_data.authority);
        if !nonce_is_authorized {
            return None;
        }

        Some((*nonce_address, nonce_account))
    }

    fn check_transaction_for_nonce(
        &self,
        tx: &SanitizedTransaction,
        next_durable_nonce: &DurableNonce,
    ) -> Option<TransactionAccount> {
        let nonce_is_advanceable = tx.message().recent_blockhash() != next_durable_nonce.as_hash();
        if nonce_is_advanceable {
            self.check_message_for_nonce(tx.message())
        } else {
            None
        }
    }

    pub fn check_transactions(
        &self,
        sanitized_txs: &[SanitizedTransaction],
        lock_results: &[Result<()>],
        max_age: usize,
        error_counters: &mut TransactionErrorMetrics,
    ) -> Vec<TransactionCheckResult> {
        let age_results =
            self.check_age(sanitized_txs.iter(), lock_results, max_age, error_counters);
        self.check_status_cache(sanitized_txs, age_results, error_counters)
    }

    pub fn collect_balances(&self, batch: &TransactionBatch) -> TransactionBalances {
        let mut balances: TransactionBalances = vec![];
        for transaction in batch.sanitized_transactions() {
            let mut transaction_balances: Vec<u64> = vec![];
            for account_key in transaction.message().account_keys().iter() {
                transaction_balances.push(self.get_balance(account_key));
            }
            balances.push(transaction_balances);
        }
        balances
    }

    /// Get any cached executors needed by the transaction
    fn get_executors(&self, accounts: &[TransactionAccount]) -> Rc<RefCell<Executors>> {
        let executable_keys: Vec<_> = accounts
            .iter()
            .filter_map(|(key, account)| {
                if account.executable() && !native_loader::check_id(account.owner()) {
                    Some(key)
                } else {
                    None
                }
            })
            .collect();

        if executable_keys.is_empty() {
            return Rc::new(RefCell::new(Executors::default()));
        }

        let executors = {
            let cache = self.cached_executors.read().unwrap();
            executable_keys
                .into_iter()
                .filter_map(|key| {
                    cache
                        .get(key)
                        .map(|executor| (*key, TransactionExecutor::new_cached(executor)))
                })
                .collect()
        };

        Rc::new(RefCell::new(executors))
    }

    /// Add executors back to the bank's cache if they were missing and not updated
    fn store_missing_executors(&self, executors: &RefCell<Executors>) {
        self.store_executors_internal(executors, |e| e.is_missing())
    }

    /// Add updated executors back to the bank's cache
    fn store_updated_executors(&self, executors: &RefCell<Executors>) {
        self.store_executors_internal(executors, |e| e.is_updated())
    }

    /// Helper to write a selection of executors to the bank's cache
    fn store_executors_internal(
        &self,
        executors: &RefCell<Executors>,
        selector: impl Fn(&TransactionExecutor) -> bool,
    ) {
        let executors = executors.borrow();
        let dirty_executors: Vec<_> = executors
            .iter()
            .filter_map(|(key, executor)| selector(executor).then(|| (key, executor.get())))
            .collect();

        if !dirty_executors.is_empty() {
            self.cached_executors.write().unwrap().put(&dirty_executors);
        }
    }

    /// Remove an executor from the bank's cache
    fn remove_executor(&self, pubkey: &Pubkey) {
        let _ = self.cached_executors.write().unwrap().remove(pubkey);
    }

    pub fn clear_executors(&self) {
        self.cached_executors.write().unwrap().clear();
    }

    /// Execute a transaction using the provided loaded accounts and update
    /// the executors cache if the transaction was successful.
    #[allow(clippy::too_many_arguments)]
    fn execute_loaded_transaction(
        &self,
        tx: &SanitizedTransaction,
        loaded_transaction: &mut LoadedTransaction,
        compute_budget: ComputeBudget,
        durable_nonce_fee: Option<DurableNonceFee>,
        enable_cpi_recording: bool,
        enable_log_recording: bool,
        enable_return_data_recording: bool,
        timings: &mut ExecuteTimings,
        error_counters: &mut TransactionErrorMetrics,
        log_messages_bytes_limit: Option<usize>,
    ) -> TransactionExecutionResult {
        let mut get_executors_time = Measure::start("get_executors_time");
        let executors = self.get_executors(&loaded_transaction.accounts);
        get_executors_time.stop();
        saturating_add_assign!(
            timings.execute_accessories.get_executors_us,
            get_executors_time.as_us()
        );

        let prev_accounts_data_len = self.load_accounts_data_size();
        let transaction_accounts = std::mem::take(&mut loaded_transaction.accounts);
        let mut transaction_context = TransactionContext::new(
            transaction_accounts,
            if self
                .feature_set
                .is_active(&enable_early_verification_of_account_modifications::id())
            {
                Some(self.rent_collector.rent)
            } else {
                None
            },
            compute_budget.max_invoke_depth.saturating_add(1),
            tx.message().instructions().len(),
        );

        let pre_account_state_info =
            self.get_transaction_account_state_info(&transaction_context, tx.message());

        let log_collector = if enable_log_recording {
            match log_messages_bytes_limit {
                None => Some(LogCollector::new_ref()),
                Some(log_messages_bytes_limit) => Some(LogCollector::new_ref_with_limit(Some(
                    log_messages_bytes_limit,
                ))),
            }
        } else {
            None
        };

        let (blockhash, lamports_per_signature) = self.last_blockhash_and_lamports_per_signature();

        let mut executed_units = 0u64;

        let mut process_message_time = Measure::start("process_message_time");
        let process_result = MessageProcessor::process_message(
            &self.builtin_programs.vec,
            tx.message(),
            &loaded_transaction.program_indices,
            &mut transaction_context,
            self.rent_collector.rent,
            log_collector.clone(),
            executors.clone(),
            self.feature_set.clone(),
            compute_budget,
            timings,
            &*self.sysvar_cache.read().unwrap(),
            blockhash,
            lamports_per_signature,
            prev_accounts_data_len,
            &mut executed_units,
        );
        process_message_time.stop();

        saturating_add_assign!(
            timings.execute_accessories.process_message_us,
            process_message_time.as_us()
        );

        let mut store_missing_executors_time = Measure::start("store_missing_executors_time");
        self.store_missing_executors(&executors);
        store_missing_executors_time.stop();
        saturating_add_assign!(
            timings.execute_accessories.update_executors_us,
            store_missing_executors_time.as_us()
        );

        let status = process_result
            .and_then(|info| {
                let post_account_state_info =
                    self.get_transaction_account_state_info(&transaction_context, tx.message());
                self.verify_transaction_account_state_changes(
                    &pre_account_state_info,
                    &post_account_state_info,
                    &transaction_context,
                )
                .map(|_| info)
            })
            .map_err(|err| {
                match err {
                    TransactionError::InvalidRentPayingAccount
                    | TransactionError::InsufficientFundsForRent { .. } => {
                        error_counters.invalid_rent_paying_account += 1;
                    }
                    TransactionError::InvalidAccountIndex => {
                        error_counters.invalid_account_index += 1;
                    }
                    _ => {
                        error_counters.instruction_error += 1;
                    }
                }
                err
            });
        let mut accounts_data_len_delta = status
            .as_ref()
            .map_or(0, |info| info.accounts_data_len_delta);
        let status = status.map(|_| ());

        let log_messages: Option<TransactionLogMessages> =
            log_collector.and_then(|log_collector| {
                Rc::try_unwrap(log_collector)
                    .map(|log_collector| log_collector.into_inner().into())
                    .ok()
            });

        let ExecutionRecord {
            accounts,
            instruction_trace,
            mut return_data,
            changed_account_count,
            total_size_of_all_accounts,
            total_size_of_touched_accounts,
            accounts_resize_delta,
        } = transaction_context.into();
        loaded_transaction.accounts = accounts;
        if self
            .feature_set
            .is_active(&enable_early_verification_of_account_modifications::id())
        {
            saturating_add_assign!(
                timings.details.total_account_count,
                loaded_transaction.accounts.len() as u64
            );
            saturating_add_assign!(timings.details.changed_account_count, changed_account_count);
            saturating_add_assign!(
                timings.details.total_data_size,
                total_size_of_all_accounts as usize
            );
            saturating_add_assign!(
                timings.details.data_size_changed,
                total_size_of_touched_accounts as usize
            );
            accounts_data_len_delta = status.as_ref().map_or(0, |_| accounts_resize_delta);
        }

        let inner_instructions = if enable_cpi_recording {
            Some(inner_instructions_list_from_instruction_trace(
                &instruction_trace,
            ))
        } else {
            None
        };

        let return_data = if enable_return_data_recording {
            if let Some(end_index) = return_data.data.iter().rposition(|&x| x != 0) {
                let end_index = end_index.saturating_add(1);
                return_data.data.truncate(end_index);
                Some(return_data)
            } else {
                None
            }
        } else {
            None
        };

        TransactionExecutionResult::Executed {
            details: TransactionExecutionDetails {
                status,
                log_messages,
                inner_instructions,
                durable_nonce_fee,
                return_data,
                executed_units,
                accounts_data_len_delta,
            },
            executors,
        }
    }

    #[allow(clippy::type_complexity)]
    pub fn load_and_execute_transactions(
        &self,
        batch: &TransactionBatch,
        max_age: usize,
        enable_cpi_recording: bool,
        enable_log_recording: bool,
        enable_return_data_recording: bool,
        timings: &mut ExecuteTimings,
        account_overrides: Option<&AccountOverrides>,
        log_messages_bytes_limit: Option<usize>,
    ) -> LoadAndExecuteTransactionsOutput {
        let sanitized_txs = batch.sanitized_transactions();
        debug!("processing transactions: {}", sanitized_txs.len());
        inc_new_counter_info!("bank-process_transactions", sanitized_txs.len());
        let mut error_counters = TransactionErrorMetrics::default();

        let retryable_transaction_indexes: Vec<_> = batch
            .lock_results()
            .iter()
            .enumerate()
            .filter_map(|(index, res)| match res {
                Err(TransactionError::AccountInUse) => {
                    error_counters.account_in_use += 1;
                    Some(index)
                }
                Err(TransactionError::WouldExceedMaxBlockCostLimit)
                | Err(TransactionError::WouldExceedMaxVoteCostLimit)
                | Err(TransactionError::WouldExceedMaxAccountCostLimit)
                | Err(TransactionError::WouldExceedAccountDataBlockLimit) => Some(index),
                Err(_) => None,
                Ok(_) => None,
            })
            .collect();

        let mut check_time = Measure::start("check_transactions");
        let check_results = self.check_transactions(
            sanitized_txs,
            batch.lock_results(),
            max_age,
            &mut error_counters,
        );
        check_time.stop();

        let mut load_time = Measure::start("accounts_load");
        let mut loaded_transactions = self.rc.accounts.load_accounts(
            &self.ancestors,
            sanitized_txs,
            check_results,
            &self.blockhash_queue.read().unwrap(),
            &mut error_counters,
            &self.rent_collector,
            &self.feature_set,
            &self.fee_structure,
            account_overrides,
        );
        load_time.stop();

        let mut execution_time = Measure::start("execution_time");
        let mut signature_count: u64 = 0;

        let execution_results: Vec<TransactionExecutionResult> = loaded_transactions
            .iter_mut()
            .zip(sanitized_txs.iter())
            .map(|(accs, tx)| match accs {
                (Err(e), _nonce) => TransactionExecutionResult::NotExecuted(e.clone()),
                (Ok(loaded_transaction), nonce) => {
                    let mut feature_set_clone_time = Measure::start("feature_set_clone");
                    let feature_set = self.feature_set.clone();
                    feature_set_clone_time.stop();
                    saturating_add_assign!(
                        timings.execute_accessories.feature_set_clone_us,
                        feature_set_clone_time.as_us()
                    );

                    let compute_budget = if let Some(compute_budget) = self.compute_budget {
                        compute_budget
                    } else {
                        let mut compute_budget =
                            ComputeBudget::new(compute_budget::MAX_COMPUTE_UNIT_LIMIT as u64);

                        let mut compute_budget_process_transaction_time =
                            Measure::start("compute_budget_process_transaction_time");
                        let process_transaction_result = compute_budget.process_instructions(
                            tx.message().program_instructions_iter(),
                            feature_set.is_active(&default_units_per_instruction::id()),
                            feature_set.is_active(&add_set_compute_unit_price_ix::id()),
                        );
                        compute_budget_process_transaction_time.stop();
                        saturating_add_assign!(
                            timings
                                .execute_accessories
                                .compute_budget_process_transaction_us,
                            compute_budget_process_transaction_time.as_us()
                        );
                        if let Err(err) = process_transaction_result {
                            return TransactionExecutionResult::NotExecuted(err);
                        }
                        compute_budget
                    };

                    self.execute_loaded_transaction(
                        tx,
                        loaded_transaction,
                        compute_budget,
                        nonce.as_ref().map(DurableNonceFee::from),
                        enable_cpi_recording,
                        enable_log_recording,
                        enable_return_data_recording,
                        timings,
                        &mut error_counters,
                        log_messages_bytes_limit,
                    )
                }
            })
            .collect();

        execution_time.stop();

        debug!(
            "check: {}us load: {}us execute: {}us txs_len={}",
            check_time.as_us(),
            load_time.as_us(),
            execution_time.as_us(),
            sanitized_txs.len(),
        );

        timings.saturating_add_in_place(ExecuteTimingType::CheckUs, check_time.as_us());
        timings.saturating_add_in_place(ExecuteTimingType::LoadUs, load_time.as_us());
        timings.saturating_add_in_place(ExecuteTimingType::ExecuteUs, execution_time.as_us());

        let mut executed_transactions_count: usize = 0;
        let mut executed_with_successful_result_count: usize = 0;
        let err_count = &mut error_counters.total;
        let transaction_log_collector_config =
            self.transaction_log_collector_config.read().unwrap();

        let mut collect_logs_time = Measure::start("collect_logs_time");
        for (execution_result, tx) in execution_results.iter().zip(sanitized_txs) {
            if let Some(debug_keys) = &self.transaction_debug_keys {
                for key in tx.message().account_keys().iter() {
                    if debug_keys.contains(key) {
                        let result = execution_result.flattened_result();
                        info!("slot: {} result: {:?} tx: {:?}", self.slot, result, tx);
                        break;
                    }
                }
            }

            if execution_result.was_executed() // Skip log collection for unprocessed transactions
                && transaction_log_collector_config.filter != TransactionLogCollectorFilter::None
            {
                let mut filtered_mentioned_addresses = Vec::new();
                if !transaction_log_collector_config
                    .mentioned_addresses
                    .is_empty()
                {
                    for key in tx.message().account_keys().iter() {
                        if transaction_log_collector_config
                            .mentioned_addresses
                            .contains(key)
                        {
                            filtered_mentioned_addresses.push(*key);
                        }
                    }
                }

                let is_vote = vote_parser::is_simple_vote_transaction(tx);
                let store = match transaction_log_collector_config.filter {
                    TransactionLogCollectorFilter::All => {
                        !is_vote || !filtered_mentioned_addresses.is_empty()
                    }
                    TransactionLogCollectorFilter::AllWithVotes => true,
                    TransactionLogCollectorFilter::None => false,
                    TransactionLogCollectorFilter::OnlyMentionedAddresses => {
                        !filtered_mentioned_addresses.is_empty()
                    }
                };

                if store {
                    if let Some(TransactionExecutionDetails {
                        status,
                        log_messages: Some(log_messages),
                        ..
                    }) = execution_result.details()
                    {
                        let mut transaction_log_collector =
                            self.transaction_log_collector.write().unwrap();
                        let transaction_log_index = transaction_log_collector.logs.len();

                        transaction_log_collector.logs.push(TransactionLogInfo {
                            signature: *tx.signature(),
                            result: status.clone(),
                            is_vote,
                            log_messages: log_messages.clone(),
                        });
                        for key in filtered_mentioned_addresses.into_iter() {
                            transaction_log_collector
                                .mentioned_address_map
                                .entry(key)
                                .or_default()
                                .push(transaction_log_index);
                        }
                    }
                }
            }

            if execution_result.was_executed() {
                // Signature count must be accumulated only if the transaction
                // is executed, otherwise a mismatched count between banking and
                // replay could occur
                signature_count += u64::from(tx.message().header().num_required_signatures);
                executed_transactions_count += 1;
            }

            match execution_result.flattened_result() {
                Ok(()) => {
                    executed_with_successful_result_count += 1;
                }
                Err(err) => {
                    if *err_count == 0 {
                        debug!("tx error: {:?} {:?}", err, tx);
                    }
                    *err_count += 1;
                }
            }
        }
        collect_logs_time.stop();
        timings
            .saturating_add_in_place(ExecuteTimingType::CollectLogsUs, collect_logs_time.as_us());

        if *err_count > 0 {
            debug!(
                "{} errors of {} txs",
                *err_count,
                *err_count + executed_with_successful_result_count
            );
        }
        LoadAndExecuteTransactionsOutput {
            loaded_transactions,
            execution_results,
            retryable_transaction_indexes,
            executed_transactions_count,
            executed_with_successful_result_count,
            signature_count,
            error_counters,
        }
    }

    /// The maximum allowed size, in bytes, of the accounts data
    pub fn accounts_data_size_limit(&self) -> u64 {
        MAX_ACCOUNTS_DATA_LEN
    }

    /// Load the accounts data size, in bytes
    pub fn load_accounts_data_size(&self) -> u64 {
        // Mixed integer ops currently not stable, so copying the impl.
        // Copied from: https://github.com/a1phyr/rust/blob/47edde1086412b36e9efd6098b191ec15a2a760a/library/core/src/num/uint_macros.rs#L1039-L1048
        fn saturating_add_signed(lhs: u64, rhs: i64) -> u64 {
            let (res, overflow) = lhs.overflowing_add(rhs as u64);
            if overflow == (rhs < 0) {
                res
            } else if overflow {
                u64::MAX
            } else {
                u64::MIN
            }
        }
        saturating_add_signed(
            self.accounts_data_size_initial,
            self.load_accounts_data_size_delta(),
        )
    }

    /// Load the change in accounts data size in this Bank, in bytes
    pub fn load_accounts_data_size_delta(&self) -> i64 {
        let delta_on_chain = self.load_accounts_data_size_delta_on_chain();
        let delta_off_chain = self.load_accounts_data_size_delta_off_chain();
        delta_on_chain.saturating_add(delta_off_chain)
    }

    /// Load the change in accounts data size in this Bank, in bytes, from on-chain events
    /// i.e. transactions
    pub fn load_accounts_data_size_delta_on_chain(&self) -> i64 {
        self.accounts_data_size_delta_on_chain.load(Acquire)
    }

    /// Load the change in accounts data size in this Bank, in bytes, from off-chain events
    /// i.e. rent collection
    pub fn load_accounts_data_size_delta_off_chain(&self) -> i64 {
        self.accounts_data_size_delta_off_chain.load(Acquire)
    }

    /// Update the accounts data size delta from on-chain events by adding `amount`.
    /// The arithmetic saturates.
    fn update_accounts_data_size_delta_on_chain(&self, amount: i64) {
        if amount == 0 {
            return;
        }

        self.accounts_data_size_delta_on_chain
            .fetch_update(AcqRel, Acquire, |accounts_data_size_delta_on_chain| {
                Some(accounts_data_size_delta_on_chain.saturating_add(amount))
            })
            // SAFETY: unwrap() is safe since our update fn always returns `Some`
            .unwrap();
    }

    /// Update the accounts data size delta from off-chain events by adding `amount`.
    /// The arithmetic saturates.
    fn update_accounts_data_size_delta_off_chain(&self, amount: i64) {
        if amount == 0 {
            return;
        }

        self.accounts_data_size_delta_off_chain
            .fetch_update(AcqRel, Acquire, |accounts_data_size_delta_off_chain| {
                Some(accounts_data_size_delta_off_chain.saturating_add(amount))
            })
            // SAFETY: unwrap() is safe since our update fn always returns `Some`
            .unwrap();
    }

    /// Calculate the data size delta and update the off-chain accounts data size delta
    fn calculate_and_update_accounts_data_size_delta_off_chain(
        &self,
        old_data_size: usize,
        new_data_size: usize,
    ) {
        let data_size_delta = calculate_data_size_delta(old_data_size, new_data_size);
        self.update_accounts_data_size_delta_off_chain(data_size_delta);
    }

    /// Set the initial accounts data size
    /// NOTE: This fn is *ONLY FOR TESTS*
    pub fn set_accounts_data_size_initial_for_tests(&mut self, amount: u64) {
        self.accounts_data_size_initial = amount;
    }

    /// Update the accounts data size off-chain delta
    /// NOTE: This fn is *ONLY FOR TESTS*
    pub fn update_accounts_data_size_delta_off_chain_for_tests(&self, amount: i64) {
        self.update_accounts_data_size_delta_off_chain(amount)
    }

    fn get_num_signatures_in_message(message: &SanitizedMessage) -> u64 {
        let mut num_signatures = u64::from(message.header().num_required_signatures);
        // This next part is really calculating the number of pre-processor
        // operations being done and treating them like a signature
        for (program_id, instruction) in message.program_instructions_iter() {
            if secp256k1_program::check_id(program_id) || ed25519_program::check_id(program_id) {
                if let Some(num_verifies) = instruction.data.first() {
                    num_signatures = num_signatures.saturating_add(u64::from(*num_verifies));
                }
            }
        }
        num_signatures
    }

    fn get_num_write_locks_in_message(message: &SanitizedMessage) -> u64 {
        message
            .account_keys()
            .len()
            .saturating_sub(message.num_readonly_accounts()) as u64
    }

    /// Calculate fee for `SanitizedMessage`
    pub fn calculate_fee(
        message: &SanitizedMessage,
        lamports_per_signature: u64,
        fee_structure: &FeeStructure,
        support_set_compute_unit_price_ix: bool,
        use_default_units_per_instruction: bool,
    ) -> u64 {
        // Fee based on compute units and signatures
        const BASE_CONGESTION: f64 = 5_000.0;
        let current_congestion = BASE_CONGESTION.max(lamports_per_signature as f64);
        let congestion_multiplier = if lamports_per_signature == 0 {
            0.0 // test only
        } else {
            BASE_CONGESTION / current_congestion
        };

        let mut compute_budget = ComputeBudget::default();
        let prioritization_fee_details = compute_budget
            .process_instructions(
                message.program_instructions_iter(),
                use_default_units_per_instruction,
                support_set_compute_unit_price_ix,
            )
            .unwrap_or_default();
        let prioritization_fee = prioritization_fee_details.get_fee();
        let signature_fee = Self::get_num_signatures_in_message(message)
            .saturating_mul(fee_structure.lamports_per_signature);
        let write_lock_fee = Self::get_num_write_locks_in_message(message)
            .saturating_mul(fee_structure.lamports_per_write_lock);
        let compute_fee = fee_structure
            .compute_fee_bins
            .iter()
            .find(|bin| compute_budget.compute_unit_limit <= bin.limit)
            .map(|bin| bin.fee)
            .unwrap_or_else(|| {
                fee_structure
                    .compute_fee_bins
                    .last()
                    .map(|bin| bin.fee)
                    .unwrap_or_default()
            });

        ((prioritization_fee
            .saturating_add(signature_fee)
            .saturating_add(write_lock_fee)
            .saturating_add(compute_fee) as f64)
            * congestion_multiplier)
            .round() as u64
    }

    fn filter_program_errors_and_collect_fee(
        &self,
        txs: &[SanitizedTransaction],
        execution_results: &[TransactionExecutionResult],
    ) -> Vec<Result<()>> {
        let hash_queue = self.blockhash_queue.read().unwrap();
        let mut fees = 0;

        let results = txs
            .iter()
            .zip(execution_results)
            .map(|(tx, execution_result)| {
                let (execution_status, durable_nonce_fee) = match &execution_result {
                    TransactionExecutionResult::Executed { details, .. } => {
                        Ok((&details.status, details.durable_nonce_fee.as_ref()))
                    }
                    TransactionExecutionResult::NotExecuted(err) => Err(err.clone()),
                }?;

                let (lamports_per_signature, is_nonce) = durable_nonce_fee
                    .map(|durable_nonce_fee| durable_nonce_fee.lamports_per_signature())
                    .map(|maybe_lamports_per_signature| (maybe_lamports_per_signature, true))
                    .unwrap_or_else(|| {
                        (
                            hash_queue.get_lamports_per_signature(tx.message().recent_blockhash()),
                            false,
                        )
                    });

                let lamports_per_signature =
                    lamports_per_signature.ok_or(TransactionError::BlockhashNotFound)?;
                let fee = Self::calculate_fee(
                    tx.message(),
                    lamports_per_signature,
                    &self.fee_structure,
                    self.feature_set
                        .is_active(&add_set_compute_unit_price_ix::id()),
                    self.feature_set
                        .is_active(&use_default_units_in_fee_calculation::id()),
                );

                // In case of instruction error, even though no accounts
                // were stored we still need to charge the payer the
                // fee.
                //
                //...except nonce accounts, which already have their
                // post-load, fee deducted, pre-execute account state
                // stored
                if execution_status.is_err() && !is_nonce {
                    self.withdraw(tx.message().fee_payer(), fee)?;
                }

                fees += fee;
                Ok(())
            })
            .collect();

        self.collector_fees.fetch_add(fees, Relaxed);
        results
    }

    /// `committed_transactions_count` is the number of transactions out of `sanitized_txs`
    /// that was executed. Of those, `committed_transactions_count`,
    /// `committed_with_failure_result_count` is the number of executed transactions that returned
    /// a failure result.
    pub fn commit_transactions(
        &self,
        sanitized_txs: &[SanitizedTransaction],
        loaded_txs: &mut [TransactionLoadResult],
        execution_results: Vec<TransactionExecutionResult>,
        last_blockhash: Hash,
        lamports_per_signature: u64,
        counts: CommitTransactionCounts,
        timings: &mut ExecuteTimings,
    ) -> TransactionResults {
        assert!(
            !self.freeze_started(),
            "commit_transactions() working on a bank that is already frozen or is undergoing freezing!"
        );

        let CommitTransactionCounts {
            committed_transactions_count,
            committed_with_failure_result_count,
            signature_count,
        } = counts;

        let tx_count = if self.bank_tranaction_count_fix_enabled() {
            committed_transactions_count
        } else {
            committed_transactions_count.saturating_sub(committed_with_failure_result_count)
        };

        self.increment_transaction_count(tx_count);
        self.increment_signature_count(signature_count);

        inc_new_counter_info!(
            "bank-process_transactions-txs",
            committed_transactions_count as usize
        );
        inc_new_counter_info!("bank-process_transactions-sigs", signature_count as usize);

        if committed_with_failure_result_count > 0 {
            self.transaction_error_count
                .fetch_add(committed_with_failure_result_count, Relaxed);
        }

        // Should be equivalent to checking `committed_transactions_count > 0`
        if execution_results.iter().any(|result| result.was_executed()) {
            self.is_delta.store(true, Relaxed);
            self.transaction_entries_count.fetch_add(1, Relaxed);
            self.transactions_per_entry_max
                .fetch_max(committed_transactions_count, Relaxed);
        }

        let mut write_time = Measure::start("write_time");
        let durable_nonce = DurableNonce::from_blockhash(&last_blockhash);
        self.rc.accounts.store_cached(
            self.slot(),
            sanitized_txs,
            &execution_results,
            loaded_txs,
            &self.rent_collector,
            &durable_nonce,
            lamports_per_signature,
            self.preserve_rent_epoch_for_rent_exempt_accounts(),
        );
        let rent_debits = self.collect_rent(&execution_results, loaded_txs);

        // Cached vote and stake accounts are synchronized with accounts-db
        // after each transaction.
        let mut update_stakes_cache_time = Measure::start("update_stakes_cache_time");
        self.update_stakes_cache(sanitized_txs, &execution_results, loaded_txs);
        update_stakes_cache_time.stop();

        // once committed there is no way to unroll
        write_time.stop();
        debug!(
            "store: {}us txs_len={}",
            write_time.as_us(),
            sanitized_txs.len()
        );

        let mut store_updated_executors_time = Measure::start("store_updated_executors_time");
        for execution_result in &execution_results {
            if let TransactionExecutionResult::Executed { details, executors } = execution_result {
                if details.status.is_ok() {
                    self.store_updated_executors(executors);
                }
            }
        }
        store_updated_executors_time.stop();
        saturating_add_assign!(
            timings.execute_accessories.update_executors_us,
            store_updated_executors_time.as_us()
        );

        let accounts_data_len_delta = execution_results
            .iter()
            .filter_map(|execution_result| {
                execution_result
                    .details()
                    .map(|details| details.accounts_data_len_delta)
            })
            .sum();
        self.update_accounts_data_size_delta_on_chain(accounts_data_len_delta);

        timings.saturating_add_in_place(ExecuteTimingType::StoreUs, write_time.as_us());
        timings.saturating_add_in_place(
            ExecuteTimingType::UpdateStakesCacheUs,
            update_stakes_cache_time.as_us(),
        );

        let mut update_transaction_statuses_time = Measure::start("update_transaction_statuses");
        self.update_transaction_statuses(sanitized_txs, &execution_results);
        let fee_collection_results =
            self.filter_program_errors_and_collect_fee(sanitized_txs, &execution_results);
        update_transaction_statuses_time.stop();
        timings.saturating_add_in_place(
            ExecuteTimingType::UpdateTransactionStatuses,
            update_transaction_statuses_time.as_us(),
        );

        TransactionResults {
            fee_collection_results,
            execution_results,
            rent_debits,
        }
    }

    // Distribute collected rent fees for this slot to staked validators (excluding stakers)
    // according to stake.
    //
    // The nature of rent fee is the cost of doing business, every validator has to hold (or have
    // access to) the same list of accounts, so we pay according to stake, which is a rough proxy for
    // value to the network.
    //
    // Currently, rent distribution doesn't consider given validator's uptime at all (this might
    // change). That's because rent should be rewarded for the storage resource utilization cost.
    // It's treated differently from transaction fees, which is for the computing resource
    // utilization cost.
    //
    // We can't use collector_id (which is rotated according to stake-weighted leader schedule)
    // as an approximation to the ideal rent distribution to simplify and avoid this per-slot
    // computation for the distribution (time: N log N, space: N acct. stores; N = # of
    // validators).
    // The reason is that rent fee doesn't need to be incentivized for throughput unlike transaction
    // fees
    //
    // Ref: collect_fees
    #[allow(clippy::needless_collect)]
    fn distribute_rent_to_validators(
        &self,
        vote_accounts: &VoteAccountsHashMap,
        rent_to_be_distributed: u64,
    ) {
        let mut total_staked = 0;

        // Collect the stake associated with each validator.
        // Note that a validator may be present in this vector multiple times if it happens to have
        // more than one staked vote account somehow
        let mut validator_stakes = vote_accounts
            .iter()
            .filter_map(|(_vote_pubkey, (staked, account))| {
                if *staked == 0 {
                    None
                } else {
                    total_staked += *staked;
                    Some((account.node_pubkey()?, *staked))
                }
            })
            .collect::<Vec<(Pubkey, u64)>>();

        #[cfg(test)]
        if validator_stakes.is_empty() {
            // some tests bank.freezes() with bad staking state
            self.capitalization
                .fetch_sub(rent_to_be_distributed, Relaxed);
            return;
        }
        #[cfg(not(test))]
        assert!(!validator_stakes.is_empty());

        // Sort first by stake and then by validator identity pubkey for determinism
        validator_stakes.sort_by(|(pubkey1, staked1), (pubkey2, staked2)| {
            match staked2.cmp(staked1) {
                std::cmp::Ordering::Equal => pubkey2.cmp(pubkey1),
                other => other,
            }
        });

        let enforce_fix = self.no_overflow_rent_distribution_enabled();

        let mut rent_distributed_in_initial_round = 0;
        let validator_rent_shares = validator_stakes
            .into_iter()
            .map(|(pubkey, staked)| {
                let rent_share = if !enforce_fix {
                    (((staked * rent_to_be_distributed) as f64) / (total_staked as f64)) as u64
                } else {
                    (((staked as u128) * (rent_to_be_distributed as u128)) / (total_staked as u128))
                        .try_into()
                        .unwrap()
                };
                rent_distributed_in_initial_round += rent_share;
                (pubkey, rent_share)
            })
            .collect::<Vec<(Pubkey, u64)>>();

        // Leftover lamports after fraction calculation, will be paid to validators starting from highest stake
        // holder
        let mut leftover_lamports = rent_to_be_distributed - rent_distributed_in_initial_round;

        let mut rewards = vec![];
        validator_rent_shares
            .into_iter()
            .for_each(|(pubkey, rent_share)| {
                let rent_to_be_paid = if leftover_lamports > 0 {
                    leftover_lamports -= 1;
                    rent_share + 1
                } else {
                    rent_share
                };
                if !enforce_fix || rent_to_be_paid > 0 {
                    let mut account = self
                        .get_account_with_fixed_root(&pubkey)
                        .unwrap_or_default();
                    if account.checked_add_lamports(rent_to_be_paid).is_err() {
                        // overflow adding lamports
                        self.capitalization.fetch_sub(rent_to_be_paid, Relaxed);
                        error!(
                            "Burned {} rent lamports instead of sending to {}",
                            rent_to_be_paid, pubkey
                        );
                        inc_new_counter_error!(
                            "bank-burned_rent_lamports",
                            rent_to_be_paid as usize
                        );
                    } else {
                        self.store_account(&pubkey, &account);
                        rewards.push((
                            pubkey,
                            RewardInfo {
                                reward_type: RewardType::Rent,
                                lamports: rent_to_be_paid as i64,
                                post_balance: account.lamports(),
                                commission: None,
                            },
                        ));
                    }
                }
            });
        self.rewards.write().unwrap().append(&mut rewards);

        if enforce_fix {
            assert_eq!(leftover_lamports, 0);
        } else if leftover_lamports != 0 {
            warn!(
                "There was leftover from rent distribution: {}",
                leftover_lamports
            );
            self.capitalization.fetch_sub(leftover_lamports, Relaxed);
        }
    }

    fn distribute_rent(&self) {
        let total_rent_collected = self.collected_rent.load(Relaxed);

        let (burned_portion, rent_to_be_distributed) = self
            .rent_collector
            .rent
            .calculate_burn(total_rent_collected);

        debug!(
            "distributed rent: {} (rounded from: {}, burned: {})",
            rent_to_be_distributed, total_rent_collected, burned_portion
        );
        self.capitalization.fetch_sub(burned_portion, Relaxed);

        if rent_to_be_distributed == 0 {
            return;
        }

        self.distribute_rent_to_validators(&self.vote_accounts(), rent_to_be_distributed);
    }

    fn collect_rent(
        &self,
        execution_results: &[TransactionExecutionResult],
        loaded_txs: &mut [TransactionLoadResult],
    ) -> Vec<RentDebits> {
        let mut collected_rent: u64 = 0;
        let rent_debits: Vec<_> = loaded_txs
            .iter_mut()
            .zip(execution_results)
            .map(|((load_result, _nonce), execution_result)| {
                if let (Ok(loaded_transaction), true) =
                    (load_result, execution_result.was_executed_successfully())
                {
                    collected_rent += loaded_transaction.rent;
                    mem::take(&mut loaded_transaction.rent_debits)
                } else {
                    RentDebits::default()
                }
            })
            .collect();
        self.collected_rent.fetch_add(collected_rent, Relaxed);
        rent_debits
    }

    fn run_incinerator(&self) {
        if let Some((account, _)) =
            self.get_account_modified_since_parent_with_fixed_root(&incinerator::id())
        {
            self.capitalization.fetch_sub(account.lamports(), Relaxed);
            self.store_account(&incinerator::id(), &AccountSharedData::default());
        }
    }

    /// after deserialize, populate rewrites with accounts that would normally have had their data rewritten in this slot due to rent collection (but didn't)
    pub fn prepare_rewrites_for_hash(&self) {
        self.collect_rent_eagerly(true);
    }

    /// Get stake and stake node accounts
    pub(crate) fn get_stake_accounts(&self, minimized_account_set: &DashSet<Pubkey>) {
        self.stakes_cache
            .stakes()
            .stake_delegations()
            .iter()
            .for_each(|(pubkey, _)| {
                minimized_account_set.insert(*pubkey);
            });

        self.stakes_cache
            .stakes()
            .staked_nodes()
            .par_iter()
            .for_each(|(pubkey, _)| {
                minimized_account_set.insert(*pubkey);
            });
    }

    /// return all end partition indexes for the given partition
    /// partition could be (0, 1, N). In this case we only return [1]
    ///  the single 'end_index' that covers this partition.
    /// partition could be (0, 2, N). In this case, we return [1, 2], which are all
    /// the 'end_index' values contained in that range.
    /// (0, 0, N) returns [0] as a special case.
    /// There is a relationship between
    /// 1. 'pubkey_range_from_partition'
    /// 2. 'partition_from_pubkey'
    /// 3. this function
    fn get_partition_end_indexes(partition: &Partition) -> Vec<PartitionIndex> {
        if partition.0 == partition.1 && partition.0 == 0 {
            // special case for start=end=0. ie. (0, 0, N). This returns [0]
            vec![0]
        } else {
            // normal case of (start, end, N)
            // so, we want [start+1, start+2, ..=end]
            // if start == end, then return []
            (partition.0..partition.1).map(|index| index + 1).collect()
        }
    }

    fn collect_rent_eagerly(&self, just_rewrites: bool) {
        if self.lazy_rent_collection.load(Relaxed) {
            return;
        }

        let mut measure = Measure::start("collect_rent_eagerly-ms");
        let partitions = self.rent_collection_partitions();
        let count = partitions.len();
        let rent_metrics = RentMetrics::default();
        // partitions will usually be 1, but could be more if we skip slots
        let mut parallel = count > 1;
        if parallel {
            let ranges = partitions
                .iter()
                .map(|partition| (*partition, Self::pubkey_range_from_partition(*partition)))
                .collect::<Vec<_>>();
            // test every range to make sure ranges are not overlapping
            // some tests collect rent from overlapping ranges
            // example: [(0, 31, 32), (0, 0, 128), (0, 27, 128)]
            // read-modify-write of an account for rent collection cannot be done in parallel
            'outer: for i in 0..ranges.len() {
                for j in 0..ranges.len() {
                    if i == j {
                        continue;
                    }

                    let i = &ranges[i].1;
                    let j = &ranges[j].1;
                    // make sure i doesn't contain j
                    if i.contains(j.start()) || i.contains(j.end()) {
                        parallel = false;
                        break 'outer;
                    }
                }
            }

            if parallel {
                let thread_pool = &self.rc.accounts.accounts_db.thread_pool;
                thread_pool.install(|| {
                    ranges.into_par_iter().for_each(|range| {
                        self.collect_rent_in_range(range.0, range.1, just_rewrites, &rent_metrics)
                    });
                });
            }
        }
        if !parallel {
            // collect serially
            partitions.into_iter().for_each(|partition| {
                self.collect_rent_in_partition(partition, just_rewrites, &rent_metrics)
            });
        }
        measure.stop();
        datapoint_info!(
            "collect_rent_eagerly",
            ("accounts", rent_metrics.count.load(Relaxed), i64),
            ("partitions", count, i64),
            (
                "skipped_rewrites",
                self.rewrites_skipped_this_slot.read().unwrap().len(),
                i64
            ),
            ("total_time_us", measure.as_us(), i64),
            (
                "hold_range_us",
                rent_metrics.hold_range_us.load(Relaxed),
                i64
            ),
            ("load_us", rent_metrics.load_us.load(Relaxed), i64),
            ("collect_us", rent_metrics.collect_us.load(Relaxed), i64),
            ("hash_us", rent_metrics.hash_us.load(Relaxed), i64),
            ("store_us", rent_metrics.store_us.load(Relaxed), i64),
        );
    }

    #[cfg(test)]
    fn restore_old_behavior_for_fragile_tests(&self) {
        self.lazy_rent_collection.store(true, Relaxed);
    }

    fn rent_collection_partitions(&self) -> Vec<Partition> {
        if !self.use_fixed_collection_cycle() {
            // This mode is for production/development/testing.
            // In this mode, we iterate over the whole pubkey value range for each epochs
            // including warm-up epochs.
            // The only exception is the situation where normal epochs are relatively short
            // (currently less than 2 day). In that case, we arrange a single collection
            // cycle to be multiple of epochs so that a cycle could be greater than the 2 day.
            self.variable_cycle_partitions()
        } else {
            // This mode is mainly for benchmarking only.
            // In this mode, we always iterate over the whole pubkey value range with
            // <slot_count_in_two_day> slots as a collection cycle, regardless warm-up or
            // alignment between collection cycles and epochs.
            // Thus, we can simulate stable processing load of eager rent collection,
            // strictly proportional to the number of pubkeys since genesis.
            self.fixed_cycle_partitions()
        }
    }

    /// Collect rent from `accounts`
    ///
    /// This fn is called inside a parallel loop from `collect_rent_in_partition()`.  Avoid adding
    /// any code that causes contention on shared memory/data (i.e. do not update atomic metrics).
    ///
    /// The return value is a struct of computed values that `collect_rent_in_partition()` will
    /// reduce at the end of its parallel loop.  If possible, place data/computation that cause
    /// contention/take locks in the return struct and process them in
    /// `collect_rent_from_partition()` after reducing the parallel loop.
    fn collect_rent_from_accounts(
        &self,
        mut accounts: Vec<(Pubkey, AccountSharedData, Slot)>,
        just_rewrites: bool,
        rent_paying_pubkeys: Option<&HashSet<Pubkey>>,
        partition_index: PartitionIndex,
    ) -> CollectRentFromAccountsInfo {
        let mut rent_debits = RentDebits::default();
        let mut total_rent_collected_info = CollectedInfo::default();
        let bank_slot = self.slot();
        let mut rewrites_skipped = Vec::with_capacity(accounts.len());
        let mut accounts_to_store =
            Vec::<(&Pubkey, &AccountSharedData)>::with_capacity(accounts.len());
        let mut time_collecting_rent_us = 0;
        let mut time_hashing_skipped_rewrites_us = 0;
        let mut time_storing_accounts_us = 0;
        let can_skip_rewrites = self.rc.accounts.accounts_db.skip_rewrites || just_rewrites;
        let preserve_rent_epoch_for_rent_exempt_accounts =
            self.preserve_rent_epoch_for_rent_exempt_accounts();
        for (pubkey, account, loaded_slot) in accounts.iter_mut() {
            let old_rent_epoch = account.rent_epoch();
            let (rent_collected_info, measure) =
                measure!(self.rent_collector.collect_from_existing_account(
                    pubkey,
                    account,
                    self.rc.accounts.accounts_db.filler_account_suffix.as_ref(),
                    preserve_rent_epoch_for_rent_exempt_accounts,
                ));
            time_collecting_rent_us += measure.as_us();

            // only store accounts where we collected rent
            // but get the hash for all these accounts even if collected rent is 0 (= not updated).
            // Also, there's another subtle side-effect from this: this
            // ensures we verify the whole on-chain state (= all accounts)
            // via the bank delta hash slowly once per an epoch.
            if can_skip_rewrites
                && Self::skip_rewrite(
                    bank_slot,
                    rent_collected_info.rent_amount,
                    *loaded_slot,
                    old_rent_epoch,
                    account,
                )
            {
                // this would have been rewritten previously. Now we skip it.
                // calculate the hash that we would have gotten if we did the rewrite.
                // This will be needed to calculate the bank's hash.
                let (hash, measure) = measure!(crate::accounts_db::AccountsDb::hash_account(
                    self.slot(),
                    account,
                    pubkey
                ));
                time_hashing_skipped_rewrites_us += measure.as_us();
                rewrites_skipped.push((*pubkey, hash));
                assert_eq!(rent_collected_info, CollectedInfo::default());
            } else if !just_rewrites {
                if rent_collected_info.rent_amount > 0 {
                    if let Some(rent_paying_pubkeys) = rent_paying_pubkeys {
                        if !rent_paying_pubkeys.contains(pubkey) {
                            // inc counter instead of assert while we verify this is correct
                            inc_new_counter_info!("unexpected-rent-paying-pubkey", 1);
                            warn!(
                                "Collecting rent from unexpected pubkey: {}, slot: {}, parent_slot: {:?}, partition_index: {}, partition_from_pubkey: {}",
                                pubkey,
                                self.slot(),
                                self.parent().map(|bank| bank.slot()),
                                partition_index,
                                Bank::partition_from_pubkey(pubkey, self.epoch_schedule.slots_per_epoch),
                            );
                        }
                    }
                }
                total_rent_collected_info += rent_collected_info;
                accounts_to_store.push((pubkey, account));
            }
            rent_debits.insert(pubkey, rent_collected_info.rent_amount, account.lamports());
        }

        if !accounts_to_store.is_empty() {
            // TODO: Maybe do not call `store_accounts()` here.  Instead return `accounts_to_store`
            // and have `collect_rent_in_partition()` perform all the stores.
            let (_, measure) = measure!(self.store_accounts((self.slot(), &accounts_to_store[..])));
            time_storing_accounts_us += measure.as_us();
        }

        CollectRentFromAccountsInfo {
            rent_collected_info: total_rent_collected_info,
            rent_rewards: rent_debits.into_unordered_rewards_iter().collect(),
            rewrites_skipped,
            time_collecting_rent_us,
            time_hashing_skipped_rewrites_us,
            time_storing_accounts_us,
            num_accounts: accounts.len(),
        }
    }

    /// convert 'partition' to a pubkey range and 'collect_rent_in_range'
    fn collect_rent_in_partition(
        &self,
        partition: Partition,
        just_rewrites: bool,
        metrics: &RentMetrics,
    ) {
        let subrange_full = Self::pubkey_range_from_partition(partition);
        self.collect_rent_in_range(partition, subrange_full, just_rewrites, metrics)
    }

    /// get all pubkeys that we expect to be rent-paying or None, if this was not initialized at load time (that should only exist in test cases)
    fn get_rent_paying_pubkeys(&self, partition: &Partition) -> Option<HashSet<Pubkey>> {
        self.rc
            .accounts
            .accounts_db
            .accounts_index
            .rent_paying_accounts_by_partition
            .get()
            .and_then(|rent_paying_accounts| {
                rent_paying_accounts.is_initialized().then(|| {
                    Self::get_partition_end_indexes(partition)
                        .into_iter()
                        .flat_map(|end_index| {
                            rent_paying_accounts.get_pubkeys_in_partition_index(end_index)
                        })
                        .cloned()
                        .collect::<HashSet<_>>()
                })
            })
    }

    /// load accounts with pubkeys in 'subrange_full'
    /// collect rent and update 'account.rent_epoch' as necessary
    /// store accounts, whether rent was collected or not (depending on whether we skipping rewrites is enabled)
    /// update bank's rewrites set for all rewrites that were skipped
    /// if 'just_rewrites', function will only update bank's rewrites set and not actually store any accounts.
    ///  This flag is used when restoring from a snapshot to calculate and verify the initial bank's delta hash.
    fn collect_rent_in_range(
        &self,
        partition: Partition,
        subrange_full: RangeInclusive<Pubkey>,
        just_rewrites: bool,
        metrics: &RentMetrics,
    ) {
        let mut hold_range = Measure::start("hold_range");
        let thread_pool = &self.rc.accounts.accounts_db.thread_pool;
        thread_pool.install(|| {
            self.rc
                .accounts
                .hold_range_in_memory(&subrange_full, true, thread_pool);
            hold_range.stop();
            metrics.hold_range_us.fetch_add(hold_range.as_us(), Relaxed);

            let rent_paying_pubkeys_ = self.get_rent_paying_pubkeys(&partition);
            let rent_paying_pubkeys = rent_paying_pubkeys_.as_ref();

            // divide the range into num_threads smaller ranges and process in parallel
            // Note that 'pubkey_range_from_partition' cannot easily be re-used here to break the range smaller.
            // It has special handling of 0..0 and partition_count changes affect all ranges unevenly.
            let num_threads = crate::accounts_db::quarter_thread_count() as u64;
            let sz = std::mem::size_of::<u64>();
            let start_prefix = Self::prefix_from_pubkey(subrange_full.start());
            let end_prefix_inclusive = Self::prefix_from_pubkey(subrange_full.end());
            let range = end_prefix_inclusive - start_prefix;
            let increment = range / num_threads;
            let mut results = (0..num_threads)
                .into_par_iter()
                .map(|chunk| {
                    let offset = |chunk| start_prefix + chunk * increment;
                    let start = offset(chunk);
                    let last = chunk == num_threads - 1;
                    let merge_prefix = |prefix: u64, mut bound: Pubkey| {
                        bound.as_mut()[0..sz].copy_from_slice(&prefix.to_be_bytes());
                        bound
                    };
                    let start = merge_prefix(start, *subrange_full.start());
                    let (accounts, measure_load_accounts) = measure!(if last {
                        let end = *subrange_full.end();
                        let subrange = start..=end; // IN-clusive
                        self.rc
                            .accounts
                            .load_to_collect_rent_eagerly(&self.ancestors, subrange)
                    } else {
                        let end = merge_prefix(offset(chunk + 1), *subrange_full.start());
                        let subrange = start..end; // EX-clusive, the next 'start' will be this same value
                        self.rc
                            .accounts
                            .load_to_collect_rent_eagerly(&self.ancestors, subrange)
                    });
                    CollectRentInPartitionInfo::new(
                        self.collect_rent_from_accounts(
                            accounts,
                            just_rewrites,
                            rent_paying_pubkeys,
                            partition.1,
                        ),
                        Duration::from_nanos(measure_load_accounts.as_ns()),
                    )
                })
                .reduce(
                    CollectRentInPartitionInfo::default,
                    CollectRentInPartitionInfo::reduce,
                );

            // We cannot assert here that we collected from all expected keys.
            // Some accounts may have been topped off or may have had all funds removed and gone to 0 lamports.

            self.rc
                .accounts
                .hold_range_in_memory(&subrange_full, false, thread_pool);

            self.collected_rent
                .fetch_add(results.rent_collected, Relaxed);
            self.update_accounts_data_size_delta_off_chain(
                -(results.accounts_data_size_reclaimed as i64),
            );
            self.rewards
                .write()
                .unwrap()
                .append(&mut results.rent_rewards);
            self.remember_skipped_rewrites(results.rewrites_skipped);

            metrics
                .load_us
                .fetch_add(results.time_loading_accounts_us, Relaxed);
            metrics
                .collect_us
                .fetch_add(results.time_collecting_rent_us, Relaxed);
            metrics
                .hash_us
                .fetch_add(results.time_hashing_skipped_rewrites_us, Relaxed);
            metrics
                .store_us
                .fetch_add(results.time_storing_accounts_us, Relaxed);
            metrics.count.fetch_add(results.num_accounts, Relaxed);
        });
    }

    // put 'rewrites_skipped' into 'self.rewrites_skipped_this_slot'
    fn remember_skipped_rewrites(&self, rewrites_skipped: Vec<(Pubkey, Hash)>) {
        if !rewrites_skipped.is_empty() {
            let mut rewrites_skipped_this_slot = self.rewrites_skipped_this_slot.write().unwrap();
            rewrites_skipped.into_iter().for_each(|(pubkey, hash)| {
                rewrites_skipped_this_slot.insert(pubkey, hash);
            });
        }
    }

    /// return true iff storing this account is just a rewrite and can be skipped
    fn skip_rewrite(
        bank_slot: Slot,
        rent_amount: u64,
        loaded_slot: Slot,
        old_rent_epoch: Epoch,
        account: &AccountSharedData,
    ) -> bool {
        if rent_amount != 0 || account.rent_epoch() == 0 {
            // rent was != 0
            // or special case for default rent value
            // these cannot be skipped and must be written
            return false;
        }
        if old_rent_epoch != account.rent_epoch() && loaded_slot == bank_slot {
            // account's rent_epoch should increment even though we're not collecting rent.
            // and we already wrote this account in this slot, but we did not adjust rent_epoch (sys vars for example)
            // so, force ourselves to rewrite account if account was already written in this slot
            // Now, the account that was written IN this slot, where normally we would have collected rent, has the corrent 'rent_epoch'.
            // Only this last store will remain in the append vec.
            // Otherwise, later code would assume the account was written successfully in this slot with the correct 'rent_epoch'.
            return false;
        }

        // rent was 0 and no reason to rewrite, so THIS is a rewrite we can skip
        true
    }

    fn prefix_from_pubkey(pubkey: &Pubkey) -> u64 {
        const PREFIX_SIZE: usize = mem::size_of::<u64>();
        u64::from_be_bytes(pubkey.as_ref()[0..PREFIX_SIZE].try_into().unwrap())
    }

    /// This is the inverse of pubkey_range_from_partition.
    /// return the lowest end_index which would contain this pubkey
    pub fn partition_from_pubkey(
        pubkey: &Pubkey,
        partition_count: PartitionsPerCycle,
    ) -> PartitionIndex {
        type Prefix = u64;
        const PREFIX_MAX: Prefix = Prefix::max_value();

        if partition_count == 1 {
            return 0;
        }

        // not-overflowing way of `(Prefix::max_value() + 1) / partition_count`
        let partition_width = (PREFIX_MAX - partition_count + 1) / partition_count + 1;

        let prefix = Self::prefix_from_pubkey(pubkey);
        if prefix == 0 {
            return 0;
        }

        if prefix == PREFIX_MAX {
            return partition_count - 1;
        }

        let mut result = (prefix + 1) / partition_width;
        if (prefix + 1) % partition_width == 0 {
            // adjust for integer divide
            result = result.saturating_sub(1);
        }
        result
    }

    // Mostly, the pair (start_index & end_index) is equivalent to this range:
    // start_index..=end_index. But it has some exceptional cases, including
    // this important and valid one:
    //   0..=0: the first partition in the new epoch when crossing epochs
    pub fn pubkey_range_from_partition(
        (start_index, end_index, partition_count): Partition,
    ) -> RangeInclusive<Pubkey> {
        assert!(start_index <= end_index);
        assert!(start_index < partition_count);
        assert!(end_index < partition_count);
        assert!(0 < partition_count);

        type Prefix = u64;
        const PREFIX_SIZE: usize = mem::size_of::<Prefix>();
        const PREFIX_MAX: Prefix = Prefix::max_value();

        let mut start_pubkey = [0x00u8; 32];
        let mut end_pubkey = [0xffu8; 32];

        if partition_count == 1 {
            assert_eq!(start_index, 0);
            assert_eq!(end_index, 0);
            return Pubkey::new_from_array(start_pubkey)..=Pubkey::new_from_array(end_pubkey);
        }

        // not-overflowing way of `(Prefix::max_value() + 1) / partition_count`
        let partition_width = (PREFIX_MAX - partition_count + 1) / partition_count + 1;
        let mut start_key_prefix = if start_index == 0 && end_index == 0 {
            0
        } else if start_index + 1 == partition_count {
            PREFIX_MAX
        } else {
            (start_index + 1) * partition_width
        };

        let mut end_key_prefix = if end_index + 1 == partition_count {
            PREFIX_MAX
        } else {
            (end_index + 1) * partition_width - 1
        };

        if start_index != 0 && start_index == end_index {
            // n..=n (n != 0): a noop pair across epochs without a gap under
            // multi_epoch_cycle, just nullify it.
            if end_key_prefix == PREFIX_MAX {
                start_key_prefix = end_key_prefix;
                start_pubkey = end_pubkey;
            } else {
                end_key_prefix = start_key_prefix;
                end_pubkey = start_pubkey;
            }
        }

        start_pubkey[0..PREFIX_SIZE].copy_from_slice(&start_key_prefix.to_be_bytes());
        end_pubkey[0..PREFIX_SIZE].copy_from_slice(&end_key_prefix.to_be_bytes());
        let start_pubkey_final = Pubkey::new_from_array(start_pubkey);
        let end_pubkey_final = Pubkey::new_from_array(end_pubkey);
        trace!(
            "pubkey_range_from_partition: ({}-{})/{} [{}]: {}-{}",
            start_index,
            end_index,
            partition_count,
            (end_key_prefix - start_key_prefix),
            start_pubkey.iter().map(|x| format!("{:02x}", x)).join(""),
            end_pubkey.iter().map(|x| format!("{:02x}", x)).join(""),
        );
        #[cfg(test)]
        if start_index != end_index {
            assert_eq!(
                if start_index == 0 && end_index == 0 {
                    0
                } else {
                    start_index + 1
                },
                Self::partition_from_pubkey(&start_pubkey_final, partition_count),
                "{}, {}, start_key_prefix: {}, {}, {}",
                start_index,
                end_index,
                start_key_prefix,
                start_pubkey_final,
                partition_count
            );
            assert_eq!(
                end_index,
                Self::partition_from_pubkey(&end_pubkey_final, partition_count),
                "{}, {}, {}, {}",
                start_index,
                end_index,
                end_pubkey_final,
                partition_count
            );
            if start_index != 0 {
                start_pubkey[0..PREFIX_SIZE]
                    .copy_from_slice(&start_key_prefix.saturating_sub(1).to_be_bytes());
                let pubkey_test = Pubkey::new_from_array(start_pubkey);
                assert_eq!(
                    start_index,
                    Self::partition_from_pubkey(&pubkey_test, partition_count),
                    "{}, {}, start_key_prefix-1: {}, {}, {}",
                    start_index,
                    end_index,
                    start_key_prefix.saturating_sub(1),
                    pubkey_test,
                    partition_count
                );
            }
            if end_index != partition_count - 1 && end_index != 0 {
                end_pubkey[0..PREFIX_SIZE]
                    .copy_from_slice(&end_key_prefix.saturating_add(1).to_be_bytes());
                let pubkey_test = Pubkey::new_from_array(end_pubkey);
                assert_eq!(
                    end_index.saturating_add(1),
                    Self::partition_from_pubkey(&pubkey_test, partition_count),
                    "start: {}, end: {}, pubkey: {}, partition_count: {}, prefix_before_addition: {}, prefix after: {}",
                    start_index,
                    end_index,
                    pubkey_test,
                    partition_count,
                    end_key_prefix,
                    end_key_prefix.saturating_add(1),
                );
            }
        }
        // should be an inclusive range (a closed interval) like this:
        // [0xgg00-0xhhff], [0xii00-0xjjff], ... (where 0xii00 == 0xhhff + 1)
        start_pubkey_final..=end_pubkey_final
    }

    pub fn get_partitions(
        slot: Slot,
        parent_slot: Slot,
        slot_count_in_two_day: SlotCount,
    ) -> Vec<Partition> {
        let parent_cycle = parent_slot / slot_count_in_two_day;
        let current_cycle = slot / slot_count_in_two_day;
        let mut parent_cycle_index = parent_slot % slot_count_in_two_day;
        let current_cycle_index = slot % slot_count_in_two_day;
        let mut partitions = vec![];
        if parent_cycle < current_cycle {
            if current_cycle_index > 0 {
                // generate and push gapped partitions because some slots are skipped
                let parent_last_cycle_index = slot_count_in_two_day - 1;

                // ... for parent cycle
                partitions.push((
                    parent_cycle_index,
                    parent_last_cycle_index,
                    slot_count_in_two_day,
                ));

                // ... for current cycle
                partitions.push((0, 0, slot_count_in_two_day));
            }
            parent_cycle_index = 0;
        }

        partitions.push((
            parent_cycle_index,
            current_cycle_index,
            slot_count_in_two_day,
        ));

        partitions
    }

    pub(crate) fn fixed_cycle_partitions_between_slots(
        &self,
        starting_slot: Slot,
        ending_slot: Slot,
    ) -> Vec<Partition> {
        let slot_count_in_two_day = self.slot_count_in_two_day();
        Self::get_partitions(ending_slot, starting_slot, slot_count_in_two_day)
    }

    fn fixed_cycle_partitions(&self) -> Vec<Partition> {
        self.fixed_cycle_partitions_between_slots(self.parent_slot(), self.slot())
    }

    /// used only by filler accounts in debug path
    /// previous means slot - 1, not parent
    pub fn variable_cycle_partition_from_previous_slot(
        epoch_schedule: &EpochSchedule,
        slot: Slot,
    ) -> Partition {
        // similar code to Bank::variable_cycle_partitions
        let (current_epoch, current_slot_index) = epoch_schedule.get_epoch_and_slot_index(slot);
        let (parent_epoch, mut parent_slot_index) =
            epoch_schedule.get_epoch_and_slot_index(slot.saturating_sub(1));
        let cycle_params = Self::rent_single_epoch_collection_cycle_params(
            current_epoch,
            epoch_schedule.get_slots_in_epoch(current_epoch),
        );

        if parent_epoch < current_epoch {
            parent_slot_index = 0;
        }

        let generated_for_gapped_epochs = false;
        Self::get_partition_from_slot_indexes(
            cycle_params,
            parent_slot_index,
            current_slot_index,
            generated_for_gapped_epochs,
        )
    }

    pub(crate) fn variable_cycle_partitions_between_slots(
        &self,
        starting_slot: Slot,
        ending_slot: Slot,
    ) -> Vec<Partition> {
        let (starting_epoch, mut starting_slot_index) =
            self.get_epoch_and_slot_index(starting_slot);
        let (ending_epoch, ending_slot_index) = self.get_epoch_and_slot_index(ending_slot);

        let mut partitions = vec![];
        if starting_epoch < ending_epoch {
            let slot_skipped = (ending_slot - starting_slot) > 1;
            if slot_skipped {
                // Generate special partitions because there are skipped slots
                // exactly at the epoch transition.

                let parent_last_slot_index = self.get_slots_in_epoch(starting_epoch) - 1;

                // ... for parent epoch
                partitions.push(self.partition_from_slot_indexes_with_gapped_epochs(
                    starting_slot_index,
                    parent_last_slot_index,
                    starting_epoch,
                ));

                if ending_slot_index > 0 {
                    // ... for current epoch
                    partitions.push(self.partition_from_slot_indexes_with_gapped_epochs(
                        0,
                        0,
                        ending_epoch,
                    ));
                }
            }
            starting_slot_index = 0;
        }

        partitions.push(self.partition_from_normal_slot_indexes(
            starting_slot_index,
            ending_slot_index,
            ending_epoch,
        ));

        partitions
    }

    fn variable_cycle_partitions(&self) -> Vec<Partition> {
        self.variable_cycle_partitions_between_slots(self.parent_slot(), self.slot())
    }

    fn do_partition_from_slot_indexes(
        &self,
        start_slot_index: SlotIndex,
        end_slot_index: SlotIndex,
        epoch: Epoch,
        generated_for_gapped_epochs: bool,
    ) -> Partition {
        let cycle_params = self.determine_collection_cycle_params(epoch);
        Self::get_partition_from_slot_indexes(
            cycle_params,
            start_slot_index,
            end_slot_index,
            generated_for_gapped_epochs,
        )
    }

    fn get_partition_from_slot_indexes(
        cycle_params: RentCollectionCycleParams,
        start_slot_index: SlotIndex,
        end_slot_index: SlotIndex,
        generated_for_gapped_epochs: bool,
    ) -> Partition {
        let (_, _, in_multi_epoch_cycle, _, _, partition_count) = cycle_params;

        // use common codepath for both very likely and very unlikely for the sake of minimized
        // risk of any miscalculation instead of negligibly faster computation per slot for the
        // likely case.
        let mut start_partition_index =
            Self::partition_index_from_slot_index(start_slot_index, cycle_params);
        let mut end_partition_index =
            Self::partition_index_from_slot_index(end_slot_index, cycle_params);

        // Adjust partition index for some edge cases
        let is_special_new_epoch = start_slot_index == 0 && end_slot_index != 1;
        let in_middle_of_cycle = start_partition_index > 0;
        if in_multi_epoch_cycle && is_special_new_epoch && in_middle_of_cycle {
            // Adjust slot indexes so that the final partition ranges are continuous!
            // This is need because the caller gives us off-by-one indexes when
            // an epoch boundary is crossed.
            // Usually there is no need for this adjustment because cycles are aligned
            // with epochs. But for multi-epoch cycles, adjust the indexes if it
            // happens in the middle of a cycle for both gapped and not-gapped cases:
            //
            // epoch (slot range)|slot idx.*1|raw part. idx.|adj. part. idx.|epoch boundary
            // ------------------+-----------+--------------+---------------+--------------
            // 3 (20..30)        | [7..8]    |   7.. 8      |   7.. 8
            //                   | [8..9]    |   8.. 9      |   8.. 9
            // 4 (30..40)        | [0..0]    |<10>..10      | <9>..10      <--- not gapped
            //                   | [0..1]    |  10..11      |  10..12
            //                   | [1..2]    |  11..12      |  11..12
            //                   | [2..9   *2|  12..19      |  12..19      <-+
            // 5 (40..50)        |  0..0   *2|<20>..<20>    |<19>..<19> *3 <-+- gapped
            //                   |  0..4]    |<20>..24      |<19>..24      <-+
            //                   | [4..5]    |  24..25      |  24..25
            //                   | [5..6]    |  25..26      |  25..26
            //
            // NOTE: <..> means the adjusted slots
            //
            // *1: The range of parent_bank.slot() and current_bank.slot() is firstly
            //     split by the epoch boundaries and then the split ones are given to us.
            //     The original ranges are denoted as [...]
            // *2: These are marked with generated_for_gapped_epochs = true.
            // *3: This becomes no-op partition
            start_partition_index -= 1;
            if generated_for_gapped_epochs {
                assert_eq!(start_slot_index, end_slot_index);
                end_partition_index -= 1;
            }
        }

        (start_partition_index, end_partition_index, partition_count)
    }

    fn partition_from_normal_slot_indexes(
        &self,
        start_slot_index: SlotIndex,
        end_slot_index: SlotIndex,
        epoch: Epoch,
    ) -> Partition {
        self.do_partition_from_slot_indexes(start_slot_index, end_slot_index, epoch, false)
    }

    fn partition_from_slot_indexes_with_gapped_epochs(
        &self,
        start_slot_index: SlotIndex,
        end_slot_index: SlotIndex,
        epoch: Epoch,
    ) -> Partition {
        self.do_partition_from_slot_indexes(start_slot_index, end_slot_index, epoch, true)
    }

    fn rent_single_epoch_collection_cycle_params(
        epoch: Epoch,
        slot_count_per_epoch: SlotCount,
    ) -> RentCollectionCycleParams {
        (
            epoch,
            slot_count_per_epoch,
            false,
            0,
            1,
            slot_count_per_epoch,
        )
    }

    fn determine_collection_cycle_params(&self, epoch: Epoch) -> RentCollectionCycleParams {
        let slot_count_per_epoch = self.get_slots_in_epoch(epoch);

        if !self.use_multi_epoch_collection_cycle(epoch) {
            // mnb should always go through this code path
            Self::rent_single_epoch_collection_cycle_params(epoch, slot_count_per_epoch)
        } else {
            let epoch_count_in_cycle = self.slot_count_in_two_day() / slot_count_per_epoch;
            let partition_count = slot_count_per_epoch * epoch_count_in_cycle;

            (
                epoch,
                slot_count_per_epoch,
                true,
                self.first_normal_epoch(),
                epoch_count_in_cycle,
                partition_count,
            )
        }
    }

    fn partition_index_from_slot_index(
        slot_index_in_epoch: SlotIndex,
        (
            epoch,
            slot_count_per_epoch,
            _,
            base_epoch,
            epoch_count_per_cycle,
            _,
        ): RentCollectionCycleParams,
    ) -> PartitionIndex {
        let epoch_offset = epoch - base_epoch;
        let epoch_index_in_cycle = epoch_offset % epoch_count_per_cycle;
        slot_index_in_epoch + epoch_index_in_cycle * slot_count_per_epoch
    }

    // Given short epochs, it's too costly to collect rent eagerly
    // within an epoch, so lower the frequency of it.
    // These logic isn't strictly eager anymore and should only be used
    // for development/performance purpose.
    // Absolutely not under ClusterType::MainnetBeta!!!!
    fn use_multi_epoch_collection_cycle(&self, epoch: Epoch) -> bool {
        // Force normal behavior, disabling multi epoch collection cycle for manual local testing
        #[cfg(not(test))]
        if self.slot_count_per_normal_epoch() == solana_sdk::epoch_schedule::MINIMUM_SLOTS_PER_EPOCH
        {
            return false;
        }

        epoch >= self.first_normal_epoch()
            && self.slot_count_per_normal_epoch() < self.slot_count_in_two_day()
    }

    pub(crate) fn use_fixed_collection_cycle(&self) -> bool {
        // Force normal behavior, disabling fixed collection cycle for manual local testing
        #[cfg(not(test))]
        if self.slot_count_per_normal_epoch() == solana_sdk::epoch_schedule::MINIMUM_SLOTS_PER_EPOCH
        {
            return false;
        }

        self.cluster_type() != ClusterType::MainnetBeta
            && self.slot_count_per_normal_epoch() < self.slot_count_in_two_day()
    }

    fn slot_count_in_two_day(&self) -> SlotCount {
        Self::slot_count_in_two_day_helper(self.ticks_per_slot)
    }

    // This value is specially chosen to align with slots per epoch in mainnet-beta and testnet
    // Also, assume 500GB account data set as the extreme, then for 2 day (=48 hours) to collect
    // rent eagerly, we'll consume 5.7 MB/s IO bandwidth, bidirectionally.
    pub fn slot_count_in_two_day_helper(ticks_per_slot: SlotCount) -> SlotCount {
        2 * DEFAULT_TICKS_PER_SECOND * SECONDS_PER_DAY / ticks_per_slot
    }

    fn slot_count_per_normal_epoch(&self) -> SlotCount {
        self.get_slots_in_epoch(self.first_normal_epoch())
    }

    pub fn cluster_type(&self) -> ClusterType {
        // unwrap is safe; self.cluster_type is ensured to be Some() always...
        // we only using Option here for ABI compatibility...
        self.cluster_type.unwrap()
    }

    /// Process a batch of transactions.
    #[must_use]
    pub fn load_execute_and_commit_transactions(
        &self,
        batch: &TransactionBatch,
        max_age: usize,
        collect_balances: bool,
        enable_cpi_recording: bool,
        enable_log_recording: bool,
        enable_return_data_recording: bool,
        timings: &mut ExecuteTimings,
        log_messages_bytes_limit: Option<usize>,
    ) -> (TransactionResults, TransactionBalancesSet) {
        let pre_balances = if collect_balances {
            self.collect_balances(batch)
        } else {
            vec![]
        };

        let LoadAndExecuteTransactionsOutput {
            mut loaded_transactions,
            execution_results,
            executed_transactions_count,
            executed_with_successful_result_count,
            signature_count,
            ..
        } = self.load_and_execute_transactions(
            batch,
            max_age,
            enable_cpi_recording,
            enable_log_recording,
            enable_return_data_recording,
            timings,
            None,
            log_messages_bytes_limit,
        );

        let (last_blockhash, lamports_per_signature) =
            self.last_blockhash_and_lamports_per_signature();
        let results = self.commit_transactions(
            batch.sanitized_transactions(),
            &mut loaded_transactions,
            execution_results,
            last_blockhash,
            lamports_per_signature,
            CommitTransactionCounts {
                committed_transactions_count: executed_transactions_count as u64,
                committed_with_failure_result_count: executed_transactions_count
                    .saturating_sub(executed_with_successful_result_count)
                    as u64,
                signature_count,
            },
            timings,
        );
        let post_balances = if collect_balances {
            self.collect_balances(batch)
        } else {
            vec![]
        };
        (
            results,
            TransactionBalancesSet::new(pre_balances, post_balances),
        )
    }

    /// Process a Transaction. This is used for unit tests and simply calls the vector
    /// Bank::process_transactions method.
    pub fn process_transaction(&self, tx: &Transaction) -> Result<()> {
        self.try_process_transactions(std::iter::once(tx))?[0].clone()?;
        tx.signatures
            .get(0)
            .map_or(Ok(()), |sig| self.get_signature_status(sig).unwrap())
    }

    /// Process a Transaction and store program log data. This is used for unit tests, and simply
    /// replicates the vector Bank::process_transactions method with `enable_cpi_recording: true`
    pub fn process_transaction_with_logs(&self, tx: &Transaction) -> Result<()> {
        let txs = vec![VersionedTransaction::from(tx.clone())];
        let batch = self.prepare_entry_batch(txs)?;
        let _results = self.load_execute_and_commit_transactions(
            &batch,
            MAX_PROCESSING_AGE,
            false,
            false,
            true,
            false,
            &mut ExecuteTimings::default(),
            None,
        );
        tx.signatures
            .get(0)
            .map_or(Ok(()), |sig| self.get_signature_status(sig).unwrap())
    }

    /// Process multiple transaction in a single batch. This is used for benches and unit tests.
    ///
    /// # Panics
    ///
    /// Panics if any of the transactions do not pass sanitization checks.
    #[must_use]
    pub fn process_transactions<'a>(
        &self,
        txs: impl Iterator<Item = &'a Transaction>,
    ) -> Vec<Result<()>> {
        self.try_process_transactions(txs).unwrap()
    }

    /// Process multiple transaction in a single batch. This is used for benches and unit tests.
    /// Short circuits if any of the transactions do not pass sanitization checks.
    pub fn try_process_transactions<'a>(
        &self,
        txs: impl Iterator<Item = &'a Transaction>,
    ) -> Result<Vec<Result<()>>> {
        let txs = txs
            .map(|tx| VersionedTransaction::from(tx.clone()))
            .collect();
        self.try_process_entry_transactions(txs)
    }

    /// Process entry transactions in a single batch. This is used for benches and unit tests.
    ///
    /// # Panics
    ///
    /// Panics if any of the transactions do not pass sanitization checks.
    #[must_use]
    pub fn process_entry_transactions(&self, txs: Vec<VersionedTransaction>) -> Vec<Result<()>> {
        self.try_process_entry_transactions(txs).unwrap()
    }

    /// Process multiple transaction in a single batch. This is used for benches and unit tests.
    /// Short circuits if any of the transactions do not pass sanitization checks.
    pub fn try_process_entry_transactions(
        &self,
        txs: Vec<VersionedTransaction>,
    ) -> Result<Vec<Result<()>>> {
        let batch = self.prepare_entry_batch(txs)?;
        Ok(self.process_transaction_batch(&batch))
    }

    #[must_use]
    fn process_transaction_batch(&self, batch: &TransactionBatch) -> Vec<Result<()>> {
        self.load_execute_and_commit_transactions(
            batch,
            MAX_PROCESSING_AGE,
            false,
            false,
            false,
            false,
            &mut ExecuteTimings::default(),
            None,
        )
        .0
        .fee_collection_results
    }

    /// Create, sign, and process a Transaction from `keypair` to `to` of
    /// `n` lamports where `blockhash` is the last Entry ID observed by the client.
    pub fn transfer(&self, n: u64, keypair: &Keypair, to: &Pubkey) -> Result<Signature> {
        let blockhash = self.last_blockhash();
        let tx = system_transaction::transfer(keypair, to, n, blockhash);
        let signature = tx.signatures[0];
        self.process_transaction(&tx).map(|_| signature)
    }

    pub fn read_balance(account: &AccountSharedData) -> u64 {
        account.lamports()
    }
    /// Each program would need to be able to introspect its own state
    /// this is hard-coded to the Budget language
    pub fn get_balance(&self, pubkey: &Pubkey) -> u64 {
        self.get_account(pubkey)
            .map(|x| Self::read_balance(&x))
            .unwrap_or(0)
    }

    /// Compute all the parents of the bank in order
    pub fn parents(&self) -> Vec<Arc<Bank>> {
        let mut parents = vec![];
        let mut bank = self.parent();
        while let Some(parent) = bank {
            parents.push(parent.clone());
            bank = parent.parent();
        }
        parents
    }

    /// Compute all the parents of the bank including this bank itself
    pub fn parents_inclusive(self: Arc<Self>) -> Vec<Arc<Bank>> {
        let mut parents = self.parents();
        parents.insert(0, self);
        parents
    }

    pub fn store_account<T: ReadableAccount + Sync + ZeroLamport>(
        &self,
        pubkey: &Pubkey,
        account: &T,
    ) {
        self.store_accounts((self.slot(), &[(pubkey, account)][..]))
    }

    pub fn store_accounts<'a, T: ReadableAccount + Sync + ZeroLamport>(
        &self,
        accounts: impl StorableAccounts<'a, T>,
    ) {
        assert!(!self.freeze_started());
        let mut m = Measure::start("stakes_cache.check_and_store");
        (0..accounts.len()).into_iter().for_each(|i| {
            self.stakes_cache
                .check_and_store(accounts.pubkey(i), accounts.account(i))
        });
        self.rc.accounts.store_accounts_cached(accounts);
        m.stop();
        self.rc
            .accounts
            .accounts_db
            .stats
            .stakes_cache_check_and_store_us
            .fetch_add(m.as_us(), Relaxed);
    }

    pub fn force_flush_accounts_cache(&self) {
        self.rc
            .accounts
            .accounts_db
            .flush_accounts_cache(true, Some(self.slot()))
    }

    pub fn flush_accounts_cache_if_needed(&self) {
        self.rc
            .accounts
            .accounts_db
            .flush_accounts_cache(false, Some(self.slot()))
    }

    #[cfg(test)]
    pub fn flush_accounts_cache_slot(&self) {
        self.rc
            .accounts
            .accounts_db
            .flush_accounts_cache_slot(self.slot())
    }

    pub fn expire_old_recycle_stores(&self) {
        self.rc.accounts.accounts_db.expire_old_recycle_stores()
    }

    /// Technically this issues (or even burns!) new lamports,
    /// so be extra careful for its usage
    fn store_account_and_update_capitalization(
        &self,
        pubkey: &Pubkey,
        new_account: &AccountSharedData,
    ) {
        let old_account_data_size =
            if let Some(old_account) = self.get_account_with_fixed_root(pubkey) {
                match new_account.lamports().cmp(&old_account.lamports()) {
                    std::cmp::Ordering::Greater => {
                        let increased = new_account.lamports() - old_account.lamports();
                        trace!(
                            "store_account_and_update_capitalization: increased: {} {}",
                            pubkey,
                            increased
                        );
                        self.capitalization.fetch_add(increased, Relaxed);
                    }
                    std::cmp::Ordering::Less => {
                        let decreased = old_account.lamports() - new_account.lamports();
                        trace!(
                            "store_account_and_update_capitalization: decreased: {} {}",
                            pubkey,
                            decreased
                        );
                        self.capitalization.fetch_sub(decreased, Relaxed);
                    }
                    std::cmp::Ordering::Equal => {}
                }
                old_account.data().len()
            } else {
                trace!(
                    "store_account_and_update_capitalization: created: {} {}",
                    pubkey,
                    new_account.lamports()
                );
                self.capitalization
                    .fetch_add(new_account.lamports(), Relaxed);
                0
            };

        self.store_account(pubkey, new_account);
        self.calculate_and_update_accounts_data_size_delta_off_chain(
            old_account_data_size,
            new_account.data().len(),
        );
    }

    fn withdraw(&self, pubkey: &Pubkey, lamports: u64) -> Result<()> {
        match self.get_account_with_fixed_root(pubkey) {
            Some(mut account) => {
                let min_balance = match get_system_account_kind(&account) {
                    Some(SystemAccountKind::Nonce) => self
                        .rent_collector
                        .rent
                        .minimum_balance(nonce::State::size()),
                    _ => 0,
                };

                lamports
                    .checked_add(min_balance)
                    .filter(|required_balance| *required_balance <= account.lamports())
                    .ok_or(TransactionError::InsufficientFundsForFee)?;
                account
                    .checked_sub_lamports(lamports)
                    .map_err(|_| TransactionError::InsufficientFundsForFee)?;
                self.store_account(pubkey, &account);

                Ok(())
            }
            None => Err(TransactionError::AccountNotFound),
        }
    }

    pub fn deposit(
        &self,
        pubkey: &Pubkey,
        lamports: u64,
    ) -> std::result::Result<u64, LamportsError> {
        // This doesn't collect rents intentionally.
        // Rents should only be applied to actual TXes
        let mut account = self.get_account_with_fixed_root(pubkey).unwrap_or_default();
        account.checked_add_lamports(lamports)?;
        self.store_account(pubkey, &account);
        Ok(account.lamports())
    }

    pub fn accounts(&self) -> Arc<Accounts> {
        self.rc.accounts.clone()
    }

    fn finish_init(
        &mut self,
        genesis_config: &GenesisConfig,
        additional_builtins: Option<&Builtins>,
        debug_do_not_add_builtins: bool,
    ) {
        self.rewards_pool_pubkeys =
            Arc::new(genesis_config.rewards_pools.keys().cloned().collect());

        let mut builtins = builtins::get();
        if let Some(additional_builtins) = additional_builtins {
            builtins
                .genesis_builtins
                .extend_from_slice(&additional_builtins.genesis_builtins);
            builtins
                .feature_transitions
                .extend_from_slice(&additional_builtins.feature_transitions);
        }
        if !debug_do_not_add_builtins {
            for builtin in builtins.genesis_builtins {
                self.add_builtin(
                    &builtin.name,
                    &builtin.id,
                    builtin.process_instruction_with_context,
                );
            }
            for precompile in get_precompiles() {
                if precompile.feature.is_none() {
                    self.add_precompile(&precompile.program_id);
                }
            }
        }
        self.builtin_feature_transitions = Arc::new(builtins.feature_transitions);

        self.apply_feature_activations(
            ApplyFeatureActivationsCaller::FinishInit,
            debug_do_not_add_builtins,
        );

        if self
            .feature_set
            .is_active(&feature_set::cap_accounts_data_len::id())
        {
            self.cost_tracker = RwLock::new(CostTracker::new_with_account_data_size_limit(Some(
                self.accounts_data_size_limit()
                    .saturating_sub(self.accounts_data_size_initial),
            )));
        }
    }

    pub fn set_inflation(&self, inflation: Inflation) {
        *self.inflation.write().unwrap() = inflation;
    }

    pub fn set_compute_budget(&mut self, compute_budget: Option<ComputeBudget>) {
        self.compute_budget = compute_budget;
    }

    pub fn hard_forks(&self) -> Arc<RwLock<HardForks>> {
        self.hard_forks.clone()
    }

    // Hi! leaky abstraction here....
    // try to use get_account_with_fixed_root() if it's called ONLY from on-chain runtime account
    // processing. That alternative fn provides more safety.
    pub fn get_account(&self, pubkey: &Pubkey) -> Option<AccountSharedData> {
        self.get_account_modified_slot(pubkey)
            .map(|(acc, _slot)| acc)
    }

    // Hi! leaky abstraction here....
    // use this over get_account() if it's called ONLY from on-chain runtime account
    // processing (i.e. from in-band replay/banking stage; that ensures root is *fixed* while
    // running).
    // pro: safer assertion can be enabled inside AccountsDb
    // con: panics!() if called from off-chain processing
    pub fn get_account_with_fixed_root(&self, pubkey: &Pubkey) -> Option<AccountSharedData> {
        self.load_slow_with_fixed_root(&self.ancestors, pubkey)
            .map(|(acc, _slot)| acc)
    }

    pub fn get_account_modified_slot(&self, pubkey: &Pubkey) -> Option<(AccountSharedData, Slot)> {
        self.load_slow(&self.ancestors, pubkey)
    }

    fn load_slow(
        &self,
        ancestors: &Ancestors,
        pubkey: &Pubkey,
    ) -> Option<(AccountSharedData, Slot)> {
        // get_account (= primary this fn caller) may be called from on-chain Bank code even if we
        // try hard to use get_account_with_fixed_root for that purpose...
        // so pass safer LoadHint:Unspecified here as a fallback
        self.rc.accounts.load_without_fixed_root(ancestors, pubkey)
    }

    fn load_slow_with_fixed_root(
        &self,
        ancestors: &Ancestors,
        pubkey: &Pubkey,
    ) -> Option<(AccountSharedData, Slot)> {
        match self.rc.accounts.load_with_fixed_root(ancestors, pubkey) {
            Some((mut account, storage_slot)) => {
                ExpectedRentCollection::maybe_update_rent_epoch_on_load(
                    &mut account,
                    &SlotInfoInEpoch::new_small(storage_slot),
                    &SlotInfoInEpoch::new_small(self.slot()),
                    self.epoch_schedule(),
                    self.rent_collector(),
                    pubkey,
                    &self.rewrites_skipped_this_slot,
                );

                Some((account, storage_slot))
            }
            None => None,
        }
    }

    pub fn get_program_accounts(
        &self,
        program_id: &Pubkey,
        config: &ScanConfig,
    ) -> ScanResult<Vec<TransactionAccount>> {
        self.rc
            .accounts
            .load_by_program(&self.ancestors, self.bank_id, program_id, config)
    }

    pub fn get_filtered_program_accounts<F: Fn(&AccountSharedData) -> bool>(
        &self,
        program_id: &Pubkey,
        filter: F,
        config: &ScanConfig,
    ) -> ScanResult<Vec<TransactionAccount>> {
        self.rc.accounts.load_by_program_with_filter(
            &self.ancestors,
            self.bank_id,
            program_id,
            filter,
            config,
        )
    }

    pub fn get_filtered_indexed_accounts<F: Fn(&AccountSharedData) -> bool>(
        &self,
        index_key: &IndexKey,
        filter: F,
        config: &ScanConfig,
        byte_limit_for_scan: Option<usize>,
    ) -> ScanResult<Vec<TransactionAccount>> {
        self.rc.accounts.load_by_index_key_with_filter(
            &self.ancestors,
            self.bank_id,
            index_key,
            filter,
            config,
            byte_limit_for_scan,
        )
    }

    pub fn account_indexes_include_key(&self, key: &Pubkey) -> bool {
        self.rc.accounts.account_indexes_include_key(key)
    }

    pub fn get_all_accounts_with_modified_slots(&self) -> ScanResult<Vec<PubkeyAccountSlot>> {
        self.rc.accounts.load_all(&self.ancestors, self.bank_id)
    }

    pub fn get_program_accounts_modified_since_parent(
        &self,
        program_id: &Pubkey,
    ) -> Vec<TransactionAccount> {
        self.rc
            .accounts
            .load_by_program_slot(self.slot(), Some(program_id))
    }

    pub fn get_transaction_logs(
        &self,
        address: Option<&Pubkey>,
    ) -> Option<Vec<TransactionLogInfo>> {
        self.transaction_log_collector
            .read()
            .unwrap()
            .get_logs_for_address(address)
    }

    pub fn get_all_accounts_modified_since_parent(&self) -> Vec<TransactionAccount> {
        self.rc.accounts.load_by_program_slot(self.slot(), None)
    }

    // if you want get_account_modified_since_parent without fixed_root, please define so...
    fn get_account_modified_since_parent_with_fixed_root(
        &self,
        pubkey: &Pubkey,
    ) -> Option<(AccountSharedData, Slot)> {
        let just_self: Ancestors = Ancestors::from(vec![self.slot()]);
        if let Some((account, slot)) = self.load_slow_with_fixed_root(&just_self, pubkey) {
            if slot == self.slot() {
                return Some((account, slot));
            }
        }
        None
    }

    pub fn get_largest_accounts(
        &self,
        num: usize,
        filter_by_address: &HashSet<Pubkey>,
        filter: AccountAddressFilter,
    ) -> ScanResult<Vec<(Pubkey, u64)>> {
        self.rc.accounts.load_largest_accounts(
            &self.ancestors,
            self.bank_id,
            num,
            filter_by_address,
            filter,
        )
    }

    pub fn transaction_count(&self) -> u64 {
        self.transaction_count.load(Relaxed)
    }

    pub fn transaction_error_count(&self) -> u64 {
        self.transaction_error_count.load(Relaxed)
    }

    pub fn transaction_entries_count(&self) -> u64 {
        self.transaction_entries_count.load(Relaxed)
    }

    pub fn transactions_per_entry_max(&self) -> u64 {
        self.transactions_per_entry_max.load(Relaxed)
    }

    fn increment_transaction_count(&self, tx_count: u64) {
        self.transaction_count.fetch_add(tx_count, Relaxed);
    }

    pub fn signature_count(&self) -> u64 {
        self.signature_count.load(Relaxed)
    }

    fn increment_signature_count(&self, signature_count: u64) {
        self.signature_count.fetch_add(signature_count, Relaxed);
    }

    pub fn get_signature_status_processed_since_parent(
        &self,
        signature: &Signature,
    ) -> Option<Result<()>> {
        if let Some((slot, status)) = self.get_signature_status_slot(signature) {
            if slot <= self.slot() {
                return Some(status);
            }
        }
        None
    }

    pub fn get_signature_status_with_blockhash(
        &self,
        signature: &Signature,
        blockhash: &Hash,
    ) -> Option<Result<()>> {
        let rcache = self.status_cache.read().unwrap();
        rcache
            .get_status(signature, blockhash, &self.ancestors)
            .map(|v| v.1)
    }

    pub fn get_signature_status_slot(&self, signature: &Signature) -> Option<(Slot, Result<()>)> {
        let rcache = self.status_cache.read().unwrap();
        rcache.get_status_any_blockhash(signature, &self.ancestors)
    }

    pub fn get_signature_status(&self, signature: &Signature) -> Option<Result<()>> {
        self.get_signature_status_slot(signature).map(|v| v.1)
    }

    pub fn has_signature(&self, signature: &Signature) -> bool {
        self.get_signature_status_slot(signature).is_some()
    }

    /// Hash the `accounts` HashMap. This represents a validator's interpretation
    ///  of the delta of the ledger since the last vote and up to now
    fn hash_internal_state(&self) -> Hash {
        // If there are no accounts, return the hash of the previous state and the latest blockhash
        let accounts_delta_hash = self
            .rc
            .accounts
            .bank_hash_info_at(self.slot(), &self.rewrites_skipped_this_slot);
        let mut signature_count_buf = [0u8; 8];
        LittleEndian::write_u64(&mut signature_count_buf[..], self.signature_count() as u64);

        let mut hash = hashv(&[
            self.parent_hash.as_ref(),
            accounts_delta_hash.hash.as_ref(),
            &signature_count_buf,
            self.last_blockhash().as_ref(),
        ]);

        let buf = self
            .hard_forks
            .read()
            .unwrap()
            .get_hash_data(self.slot(), self.parent_slot());
        if let Some(buf) = buf {
            let hard_forked_hash = extend_and_hash(&hash, &buf);
            warn!(
                "hard fork at slot {} by hashing {:?}: {} => {}",
                self.slot(),
                buf,
                hash,
                hard_forked_hash
            );
            hash = hard_forked_hash;
        }

        info!(
            "bank frozen: {} hash: {} accounts_delta: {} signature_count: {} last_blockhash: {} capitalization: {}",
            self.slot(),
            hash,
            accounts_delta_hash.hash,
            self.signature_count(),
            self.last_blockhash(),
            self.capitalization(),
        );

        info!(
            "accounts hash slot: {} stats: {:?}",
            self.slot(),
            accounts_delta_hash.stats,
        );
        hash
    }

    /// Recalculate the hash_internal_state from the account stores. Would be used to verify a
    /// snapshot.
    /// return true if all is good
    /// Only called from startup or test code.
    #[must_use]
    pub fn verify_bank_hash(&self, config: VerifyBankHash) -> bool {
        let accounts = &self.rc.accounts;
        // Wait until initial hash calc is complete before starting a new hash calc.
        // This should only occur when we halt at a slot in ledger-tool.
        accounts
            .accounts_db
            .verify_accounts_hash_in_bg
            .wait_for_complete();

        if config.require_rooted_bank
            && !accounts
                .accounts_db
                .accounts_index
                .is_alive_root(self.slot())
        {
            if let Some(parent) = self.parent() {
                info!("{} is not a root, so attempting to verify bank hash on parent bank at slot: {}", self.slot(), parent.slot());
                return parent.verify_bank_hash(config);
            } else {
                // this will result in mismatch errors
                // accounts hash calc doesn't include unrooted slots
                panic!("cannot verify bank hash when bank is not a root");
            }
        }
        let slot = self.slot();
        let ancestors = &self.ancestors;
        let cap = self.capitalization();
        let epoch_schedule = self.epoch_schedule();
        let rent_collector = self.rent_collector();
        if config.run_in_background {
            let ancestors = ancestors.clone();
            let accounts = Arc::clone(accounts);
            let epoch_schedule = *epoch_schedule;
            let rent_collector = rent_collector.clone();
            let accounts_ = Arc::clone(&accounts);
            accounts.accounts_db.verify_accounts_hash_in_bg.start(|| {
                Builder::new()
                    .name("solana-bg-hash-verifier".to_string())
                    .spawn(move || {
                        info!(
                            "running initial verification accounts hash calculation in background"
                        );
                        let result = accounts_.verify_bank_hash_and_lamports(
                            slot,
                            &ancestors,
                            cap,
                            config.test_hash_calculation,
                            &epoch_schedule,
                            &rent_collector,
                            config.can_cached_slot_be_unflushed,
                            config.ignore_mismatch,
                            config.store_hash_raw_data_for_debug,
                        );
                        accounts_
                            .accounts_db
                            .verify_accounts_hash_in_bg
                            .background_finished();
                        result
                    })
                    .unwrap()
            });
            true // initial result is true. We haven't failed yet. If verification fails, we'll panic from bg thread.
        } else {
            let result = accounts.verify_bank_hash_and_lamports(
                slot,
                &self.ancestors,
                cap,
                config.test_hash_calculation,
                epoch_schedule,
                rent_collector,
                config.can_cached_slot_be_unflushed,
                config.ignore_mismatch,
                config.store_hash_raw_data_for_debug,
            );
            self.set_initial_accounts_hash_verification_completed();
            result
        }
    }

    /// Specify that initial verification has completed.
    /// Called internally when verification runs in the foreground thread.
    /// Also has to be called by some tests which don't do verification on startup.
    pub fn set_initial_accounts_hash_verification_completed(&self) {
        self.rc
            .accounts
            .accounts_db
            .verify_accounts_hash_in_bg
            .verification_complete();
    }

    /// return true if bg hash verification is complete
    /// return false if bg hash verification has not completed yet
    /// if hash verification failed, a panic will occur
    pub fn has_initial_accounts_hash_verification_completed(&self) -> bool {
        self.rc
            .accounts
            .accounts_db
            .verify_accounts_hash_in_bg
            .check_complete()
    }

    pub fn get_snapshot_storages(&self, base_slot: Option<Slot>) -> SnapshotStorages {
        self.rc
            .accounts
            .accounts_db
            .get_snapshot_storages(self.slot(), base_slot, None)
            .0
    }

    #[must_use]
    fn verify_hash(&self) -> bool {
        assert!(self.is_frozen());
        let calculated_hash = self.hash_internal_state();
        let expected_hash = self.hash();

        if calculated_hash == expected_hash {
            true
        } else {
            warn!(
                "verify failed: slot: {}, {} (calculated) != {} (expected)",
                self.slot(),
                calculated_hash,
                expected_hash
            );
            false
        }
    }

    pub fn verify_transaction(
        &self,
        tx: VersionedTransaction,
        verification_mode: TransactionVerificationMode,
    ) -> Result<SanitizedTransaction> {
        let sanitized_tx = {
            let size =
                bincode::serialized_size(&tx).map_err(|_| TransactionError::SanitizeFailure)?;
            if size > PACKET_DATA_SIZE as u64 {
                return Err(TransactionError::SanitizeFailure);
            }
            let message_hash = if verification_mode == TransactionVerificationMode::FullVerification
            {
                tx.verify_and_hash_message()?
            } else {
                tx.message.hash()
            };

            SanitizedTransaction::try_create(
                tx,
                message_hash,
                None,
                self,
                self.feature_set
                    .is_active(&feature_set::require_static_program_ids_in_transaction::ID),
            )
        }?;

        if verification_mode == TransactionVerificationMode::HashAndVerifyPrecompiles
            || verification_mode == TransactionVerificationMode::FullVerification
        {
            sanitized_tx.verify_precompiles(&self.feature_set)?;
        }

        Ok(sanitized_tx)
    }

    /// only called at startup vs steady-state runtime
    fn calculate_capitalization(&self, debug_verify: bool) -> u64 {
        let can_cached_slot_be_unflushed = true; // implied yes
        self.rc.accounts.calculate_capitalization(
            &self.ancestors,
            self.slot(),
            can_cached_slot_be_unflushed,
            debug_verify,
            self.epoch_schedule(),
            &self.rent_collector,
        )
    }

    /// only called at startup vs steady-state runtime
    pub fn calculate_and_verify_capitalization(&self, debug_verify: bool) -> bool {
        let calculated = self.calculate_capitalization(debug_verify);
        let expected = self.capitalization();
        if calculated == expected {
            true
        } else {
            warn!(
                "Capitalization mismatch: calculated: {} != expected: {}",
                calculated, expected
            );
            false
        }
    }

    /// Forcibly overwrites current capitalization by actually recalculating accounts' balances.
    /// This should only be used for developing purposes.
    pub fn set_capitalization(&self) -> u64 {
        let old = self.capitalization();
        let debug_verify = true;
        self.capitalization
            .store(self.calculate_capitalization(debug_verify), Relaxed);
        old
    }

    pub fn get_accounts_hash(&self) -> Hash {
        self.rc.accounts.accounts_db.get_accounts_hash(self.slot)
    }

    pub fn get_thread_pool(&self) -> &ThreadPool {
        &self.rc.accounts.accounts_db.thread_pool_clean
    }

    pub fn load_account_into_read_cache(&self, key: &Pubkey) {
        self.rc
            .accounts
            .accounts_db
            .load_account_into_read_cache(&self.ancestors, key);
    }

    pub fn update_accounts_hash_with_index_option(
        &self,
        use_index: bool,
        mut debug_verify: bool,
        is_startup: bool,
    ) -> Hash {
        let (hash, total_lamports) = self
            .rc
            .accounts
            .accounts_db
            .update_accounts_hash_with_index_option(
                use_index,
                debug_verify,
                self.slot(),
                &self.ancestors,
                Some(self.capitalization()),
                false,
                self.epoch_schedule(),
                &self.rent_collector,
                is_startup,
            );
        if total_lamports != self.capitalization() {
            datapoint_info!(
                "capitalization_mismatch",
                ("slot", self.slot(), i64),
                ("calculated_lamports", total_lamports, i64),
                ("capitalization", self.capitalization(), i64),
            );

            if !debug_verify {
                // cap mismatch detected. It has been logged to metrics above.
                // Run both versions of the calculation to attempt to get more info.
                debug_verify = true;
                self.rc
                    .accounts
                    .accounts_db
                    .update_accounts_hash_with_index_option(
                        use_index,
                        debug_verify,
                        self.slot(),
                        &self.ancestors,
                        Some(self.capitalization()),
                        false,
                        self.epoch_schedule(),
                        &self.rent_collector,
                        is_startup,
                    );
            }

            panic!(
                "capitalization_mismatch. slot: {}, calculated_lamports: {}, capitalization: {}",
                self.slot(),
                total_lamports,
                self.capitalization()
            );
        }
        hash
    }

    pub fn update_accounts_hash(&self) -> Hash {
        self.update_accounts_hash_with_index_option(true, false, false)
    }

    /// A snapshot bank should be purged of 0 lamport accounts which are not part of the hash
    /// calculation and could shield other real accounts.
    pub fn verify_snapshot_bank(
        &self,
        test_hash_calculation: bool,
        accounts_db_skip_shrink: bool,
        last_full_snapshot_slot: Option<Slot>,
    ) -> bool {
        let mut clean_time = Measure::start("clean");
        if !accounts_db_skip_shrink && self.slot() > 0 {
            info!("cleaning..");
            self.clean_accounts(true, true, last_full_snapshot_slot);
        }
        clean_time.stop();

        let mut shrink_all_slots_time = Measure::start("shrink_all_slots");
        if !accounts_db_skip_shrink && self.slot() > 0 {
            info!("shrinking..");
            self.shrink_all_slots(true, last_full_snapshot_slot);
        }
        shrink_all_slots_time.stop();

        let (mut verify, verify_time_us) = if !self.rc.accounts.accounts_db.skip_initial_hash_calc {
            info!("verify_bank_hash..");
            let mut verify_time = Measure::start("verify_bank_hash");
            let verify = self.verify_bank_hash(VerifyBankHash {
                test_hash_calculation,
                can_cached_slot_be_unflushed: false,
                ignore_mismatch: false,
                require_rooted_bank: false,
                run_in_background: true,
                store_hash_raw_data_for_debug: false,
            });
            verify_time.stop();
            (verify, verify_time.as_us())
        } else {
            self.rc
                .accounts
                .accounts_db
                .verify_accounts_hash_in_bg
                .verification_complete();
            (true, 0)
        };

        info!("verify_hash..");
        let mut verify2_time = Measure::start("verify_hash");
        // Order and short-circuiting is significant; verify_hash requires a valid bank hash
        verify = verify && self.verify_hash();
        verify2_time.stop();

        datapoint_info!(
            "verify_snapshot_bank",
            ("clean_us", clean_time.as_us(), i64),
            ("shrink_all_slots_us", shrink_all_slots_time.as_us(), i64),
            ("verify_bank_hash_us", verify_time_us, i64),
            ("verify_hash_us", verify2_time.as_us(), i64),
        );

        verify
    }

    /// Return the number of hashes per tick
    pub fn hashes_per_tick(&self) -> &Option<u64> {
        &self.hashes_per_tick
    }

    /// Return the number of ticks per slot
    pub fn ticks_per_slot(&self) -> u64 {
        self.ticks_per_slot
    }

    /// Return the number of slots per year
    pub fn slots_per_year(&self) -> f64 {
        self.slots_per_year
    }

    /// Return the number of ticks since genesis.
    pub fn tick_height(&self) -> u64 {
        self.tick_height.load(Relaxed)
    }

    /// Return the inflation parameters of the Bank
    pub fn inflation(&self) -> Inflation {
        *self.inflation.read().unwrap()
    }

    pub fn rent_collector(&self) -> &RentCollector {
        &self.rent_collector
    }

    /// Return the total capitalization of the Bank
    pub fn capitalization(&self) -> u64 {
        self.capitalization.load(Relaxed)
    }

    /// Return this bank's max_tick_height
    pub fn max_tick_height(&self) -> u64 {
        self.max_tick_height
    }

    /// Return the block_height of this bank
    pub fn block_height(&self) -> u64 {
        self.block_height
    }

    /// Return the number of slots per epoch for the given epoch
    pub fn get_slots_in_epoch(&self, epoch: Epoch) -> u64 {
        self.epoch_schedule().get_slots_in_epoch(epoch)
    }

    /// returns the epoch for which this bank's leader_schedule_slot_offset and slot would
    ///  need to cache leader_schedule
    pub fn get_leader_schedule_epoch(&self, slot: Slot) -> Epoch {
        self.epoch_schedule().get_leader_schedule_epoch(slot)
    }

    /// a bank-level cache of vote accounts and stake delegation info
    fn update_stakes_cache(
        &self,
        txs: &[SanitizedTransaction],
        execution_results: &[TransactionExecutionResult],
        loaded_txs: &[TransactionLoadResult],
    ) {
        for (i, ((load_result, _load_nonce), tx)) in loaded_txs.iter().zip(txs).enumerate() {
            if let (Ok(loaded_transaction), true) = (
                load_result,
                execution_results[i].was_executed_successfully(),
            ) {
                // note that this could get timed to: self.rc.accounts.accounts_db.stats.stakes_cache_check_and_store_us,
                //  but this code path is captured separately in ExecuteTimingType::UpdateStakesCacheUs
                let message = tx.message();
                for (_i, (pubkey, account)) in
                    (0..message.account_keys().len()).zip(loaded_transaction.accounts.iter())
                {
                    self.stakes_cache.check_and_store(pubkey, account);
                }
            }
        }
    }

    pub fn staked_nodes(&self) -> Arc<HashMap<Pubkey, u64>> {
        self.stakes_cache.stakes().staked_nodes()
    }

    /// current vote accounts for this bank along with the stake
    ///   attributed to each account
    pub fn vote_accounts(&self) -> Arc<VoteAccountsHashMap> {
        let stakes = self.stakes_cache.stakes();
        Arc::from(stakes.vote_accounts())
    }

    /// Vote account for the given vote account pubkey.
    pub fn get_vote_account(&self, vote_account: &Pubkey) -> Option<VoteAccount> {
        let stakes = self.stakes_cache.stakes();
        let vote_account = stakes.vote_accounts().get(vote_account)?;
        Some(vote_account.clone())
    }

    /// Get the EpochStakes for a given epoch
    pub fn epoch_stakes(&self, epoch: Epoch) -> Option<&EpochStakes> {
        self.epoch_stakes.get(&epoch)
    }

    pub fn epoch_stakes_map(&self) -> &HashMap<Epoch, EpochStakes> {
        &self.epoch_stakes
    }

    pub fn epoch_staked_nodes(&self, epoch: Epoch) -> Option<Arc<HashMap<Pubkey, u64>>> {
        Some(self.epoch_stakes.get(&epoch)?.stakes().staked_nodes())
    }

    /// vote accounts for the specific epoch along with the stake
    ///   attributed to each account
    pub fn epoch_vote_accounts(&self, epoch: Epoch) -> Option<&VoteAccountsHashMap> {
        let epoch_stakes = self.epoch_stakes.get(&epoch)?.stakes();
        Some(epoch_stakes.vote_accounts().as_ref())
    }

    /// Get the fixed authorized voter for the given vote account for the
    /// current epoch
    pub fn epoch_authorized_voter(&self, vote_account: &Pubkey) -> Option<&Pubkey> {
        self.epoch_stakes
            .get(&self.epoch)
            .expect("Epoch stakes for bank's own epoch must exist")
            .epoch_authorized_voters()
            .get(vote_account)
    }

    /// Get the fixed set of vote accounts for the given node id for the
    /// current epoch
    pub fn epoch_vote_accounts_for_node_id(&self, node_id: &Pubkey) -> Option<&NodeVoteAccounts> {
        self.epoch_stakes
            .get(&self.epoch)
            .expect("Epoch stakes for bank's own epoch must exist")
            .node_id_to_vote_accounts()
            .get(node_id)
    }

    /// Get the fixed total stake of all vote accounts for current epoch
    pub fn total_epoch_stake(&self) -> u64 {
        self.epoch_stakes
            .get(&self.epoch)
            .expect("Epoch stakes for bank's own epoch must exist")
            .total_stake()
    }

    /// Get the fixed stake of the given vote account for the current epoch
    pub fn epoch_vote_account_stake(&self, vote_account: &Pubkey) -> u64 {
        *self
            .epoch_vote_accounts(self.epoch())
            .expect("Bank epoch vote accounts must contain entry for the bank's own epoch")
            .get(vote_account)
            .map(|(stake, _)| stake)
            .unwrap_or(&0)
    }

    /// given a slot, return the epoch and offset into the epoch this slot falls
    /// e.g. with a fixed number for slots_per_epoch, the calculation is simply:
    ///
    ///  ( slot/slots_per_epoch, slot % slots_per_epoch )
    ///
    pub fn get_epoch_and_slot_index(&self, slot: Slot) -> (Epoch, SlotIndex) {
        self.epoch_schedule().get_epoch_and_slot_index(slot)
    }

    pub fn get_epoch_info(&self) -> EpochInfo {
        let absolute_slot = self.slot();
        let block_height = self.block_height();
        let (epoch, slot_index) = self.get_epoch_and_slot_index(absolute_slot);
        let slots_in_epoch = self.get_slots_in_epoch(epoch);
        let transaction_count = Some(self.transaction_count());
        EpochInfo {
            epoch,
            slot_index,
            slots_in_epoch,
            absolute_slot,
            block_height,
            transaction_count,
        }
    }

    pub fn is_empty(&self) -> bool {
        !self.is_delta.load(Relaxed)
    }

    /// Add an instruction processor to intercept instructions before the dynamic loader.
    pub fn add_builtin(
        &mut self,
        name: &str,
        program_id: &Pubkey,
        process_instruction: ProcessInstructionWithContext,
    ) {
        debug!("Adding program {} under {:?}", name, program_id);
        self.add_builtin_account(name, program_id, false);
        if let Some(entry) = self
            .builtin_programs
            .vec
            .iter_mut()
            .find(|entry| entry.program_id == *program_id)
        {
            entry.process_instruction = process_instruction;
        } else {
            self.builtin_programs.vec.push(BuiltinProgram {
                program_id: *program_id,
                process_instruction,
            });
        }
        debug!("Added program {} under {:?}", name, program_id);
    }

    /// Remove a builtin instruction processor if it already exists
    pub fn remove_builtin(&mut self, program_id: &Pubkey) {
        debug!("Removing program {}", program_id);
        // Don't remove the account since the bank expects the account state to
        // be idempotent
        if let Some(position) = self
            .builtin_programs
            .vec
            .iter()
            .position(|entry| entry.program_id == *program_id)
        {
            self.builtin_programs.vec.remove(position);
        }
        debug!("Removed program {}", program_id);
    }

    pub fn add_precompile(&mut self, program_id: &Pubkey) {
        debug!("Adding precompiled program {}", program_id);
        self.add_precompiled_account(program_id);
        debug!("Added precompiled program {:?}", program_id);
    }

    pub fn clean_accounts(
        &self,
        skip_last: bool,
        is_startup: bool,
        last_full_snapshot_slot: Option<Slot>,
    ) {
        // Don't clean the slot we're snapshotting because it may have zero-lamport
        // accounts that were included in the bank delta hash when the bank was frozen,
        // and if we clean them here, any newly created snapshot's hash for this bank
        // may not match the frozen hash.
        //
        // So when we're snapshotting, set `skip_last` to true so the highest slot to clean is
        // lowered by one.
        let highest_slot_to_clean = skip_last.then(|| self.slot().saturating_sub(1));

        self.rc.accounts.accounts_db.clean_accounts(
            highest_slot_to_clean,
            is_startup,
            last_full_snapshot_slot,
        );
    }

    pub fn shrink_all_slots(&self, is_startup: bool, last_full_snapshot_slot: Option<Slot>) {
        self.rc
            .accounts
            .accounts_db
            .shrink_all_slots(is_startup, last_full_snapshot_slot);
    }

    pub fn print_accounts_stats(&self) {
        self.rc.accounts.accounts_db.print_accounts_stats("");
    }

    pub fn process_stale_slot_with_budget(
        &self,
        mut consumed_budget: usize,
        budget_recovery_delta: usize,
    ) -> usize {
        if consumed_budget == 0 {
            let shrunken_account_count = self.rc.accounts.accounts_db.process_stale_slot_v1();
            if shrunken_account_count > 0 {
                datapoint_info!(
                    "stale_slot_shrink",
                    ("accounts", shrunken_account_count, i64)
                );
                consumed_budget += shrunken_account_count;
            }
        }
        consumed_budget.saturating_sub(budget_recovery_delta)
    }

    pub fn bank_tranaction_count_fix_enabled(&self) -> bool {
        self.feature_set
            .is_active(&feature_set::bank_tranaction_count_fix::id())
    }

    pub fn shrink_candidate_slots(&self) -> usize {
        self.rc.accounts.accounts_db.shrink_candidate_slots()
    }

    pub fn no_overflow_rent_distribution_enabled(&self) -> bool {
        self.feature_set
            .is_active(&feature_set::no_overflow_rent_distribution::id())
    }

    pub fn versioned_tx_message_enabled(&self) -> bool {
        self.feature_set
            .is_active(&feature_set::versioned_tx_message_enabled::id())
    }

    pub fn credits_auto_rewind(&self) -> bool {
        self.feature_set
            .is_active(&feature_set::credits_auto_rewind::id())
    }

    pub fn send_to_tpu_vote_port_enabled(&self) -> bool {
        self.feature_set
            .is_active(&feature_set::send_to_tpu_vote_port::id())
    }

    fn preserve_rent_epoch_for_rent_exempt_accounts(&self) -> bool {
        self.feature_set
            .is_active(&feature_set::preserve_rent_epoch_for_rent_exempt_accounts::id())
    }

    pub fn concurrent_replay_of_forks(&self) -> bool {
        self.feature_set
            .is_active(&feature_set::concurrent_replay_of_forks::id())
    }

    pub fn read_cost_tracker(&self) -> LockResult<RwLockReadGuard<CostTracker>> {
        self.cost_tracker.read()
    }

    pub fn write_cost_tracker(&self) -> LockResult<RwLockWriteGuard<CostTracker>> {
        self.cost_tracker.write()
    }

    // Check if the wallclock time from bank creation to now has exceeded the allotted
    // time for transaction processing
    pub fn should_bank_still_be_processing_txs(
        bank_creation_time: &Instant,
        max_tx_ingestion_nanos: u128,
    ) -> bool {
        // Do this check outside of the poh lock, hence not a method on PohRecorder
        bank_creation_time.elapsed().as_nanos() <= max_tx_ingestion_nanos
    }

    pub fn deactivate_feature(&mut self, id: &Pubkey) {
        let mut feature_set = Arc::make_mut(&mut self.feature_set).clone();
        feature_set.active.remove(id);
        feature_set.inactive.insert(*id);
        self.feature_set = Arc::new(feature_set);
    }

    pub fn activate_feature(&mut self, id: &Pubkey) {
        let mut feature_set = Arc::make_mut(&mut self.feature_set).clone();
        feature_set.inactive.remove(id);
        feature_set.active.insert(*id, 0);
        self.feature_set = Arc::new(feature_set);
    }

    pub fn fill_bank_with_ticks_for_tests(&self) {
        if self.tick_height.load(Relaxed) < self.max_tick_height {
            let last_blockhash = self.last_blockhash();
            while self.last_blockhash() == last_blockhash {
                self.register_tick(&Hash::new_unique())
            }
        } else {
            warn!("Bank already reached max tick height, cannot fill it with more ticks");
        }
    }

    // This is called from snapshot restore AND for each epoch boundary
    // The entire code path herein must be idempotent
    fn apply_feature_activations(
        &mut self,
        caller: ApplyFeatureActivationsCaller,
        debug_do_not_add_builtins: bool,
    ) {
        use ApplyFeatureActivationsCaller::*;
        let allow_new_activations = match caller {
            FinishInit => false,
            NewFromParent => true,
            WarpFromParent => false,
        };
        let new_feature_activations = self.compute_active_feature_set(allow_new_activations);

        if new_feature_activations.contains(&feature_set::pico_inflation::id()) {
            *self.inflation.write().unwrap() = Inflation::pico();
            self.fee_rate_governor.burn_percent = 50; // 50% fee burn
            self.rent_collector.rent.burn_percent = 50; // 50% rent burn
        }

        if !new_feature_activations.is_disjoint(&self.feature_set.full_inflation_features_enabled())
        {
            *self.inflation.write().unwrap() = Inflation::full();
            self.fee_rate_governor.burn_percent = 50; // 50% fee burn
            self.rent_collector.rent.burn_percent = 50; // 50% rent burn
        }

        if new_feature_activations.contains(&feature_set::spl_token_v3_4_0::id()) {
            self.replace_program_account(
                &inline_spl_token::id(),
                &inline_spl_token::program_v3_4_0::id(),
                "bank-apply_spl_token_v3_4_0",
            );
        }

        if new_feature_activations.contains(&feature_set::spl_associated_token_account_v1_1_0::id())
        {
            self.replace_program_account(
                &inline_spl_associated_token_account::id(),
                &inline_spl_associated_token_account::program_v1_1_0::id(),
                "bank-apply_spl_associated_token_account_v1_1_0",
            );
        }

        if !debug_do_not_add_builtins {
            self.apply_builtin_program_feature_transitions(
                allow_new_activations,
                &new_feature_activations,
            );
            self.reconfigure_token2_native_mint();
        }
        self.ensure_no_storage_rewards_pool();

        if new_feature_activations.contains(&feature_set::cap_accounts_data_len::id()) {
            const ACCOUNTS_DATA_LEN: u64 = 50_000_000_000;
            self.accounts_data_size_initial = ACCOUNTS_DATA_LEN;
        }
    }

    fn adjust_sysvar_balance_for_rent(&self, account: &mut AccountSharedData) {
        account.set_lamports(
            self.get_minimum_balance_for_rent_exemption(account.data().len())
                .max(account.lamports()),
        );
    }

    // Compute the active feature set based on the current bank state, and return the set of newly activated features
    fn compute_active_feature_set(&mut self, allow_new_activations: bool) -> HashSet<Pubkey> {
        let mut active = self.feature_set.active.clone();
        let mut inactive = HashSet::new();
        let mut newly_activated = HashSet::new();
        let slot = self.slot();

        for feature_id in &self.feature_set.inactive {
            let mut activated = None;
            if let Some(mut account) = self.get_account_with_fixed_root(feature_id) {
                if let Some(mut feature) = feature::from_account(&account) {
                    match feature.activated_at {
                        None => {
                            if allow_new_activations {
                                // Feature has been requested, activate it now
                                feature.activated_at = Some(slot);
                                if feature::to_account(&feature, &mut account).is_some() {
                                    self.store_account(feature_id, &account);
                                }
                                newly_activated.insert(*feature_id);
                                activated = Some(slot);
                                info!("Feature {} activated at slot {}", feature_id, slot);
                            }
                        }
                        Some(activation_slot) => {
                            if slot >= activation_slot {
                                // Feature is already active
                                activated = Some(activation_slot);
                            }
                        }
                    }
                }
            }
            if let Some(slot) = activated {
                active.insert(*feature_id, slot);
            } else {
                inactive.insert(*feature_id);
            }
        }

        self.feature_set = Arc::new(FeatureSet { active, inactive });
        newly_activated
    }

    fn apply_builtin_program_feature_transitions(
        &mut self,
        only_apply_transitions_for_new_features: bool,
        new_feature_activations: &HashSet<Pubkey>,
    ) {
        let feature_set = self.feature_set.clone();
        let should_apply_action_for_feature_transition = |feature_id: &Pubkey| -> bool {
            if only_apply_transitions_for_new_features {
                new_feature_activations.contains(feature_id)
            } else {
                feature_set.is_active(feature_id)
            }
        };

        let builtin_feature_transitions = self.builtin_feature_transitions.clone();
        for transition in builtin_feature_transitions.iter() {
            if let Some(builtin_action) =
                transition.to_action(&should_apply_action_for_feature_transition)
            {
                match builtin_action {
                    BuiltinAction::Add(builtin) => self.add_builtin(
                        &builtin.name,
                        &builtin.id,
                        builtin.process_instruction_with_context,
                    ),
                    BuiltinAction::Remove(program_id) => self.remove_builtin(&program_id),
                }
            }
        }

        for precompile in get_precompiles() {
            #[allow(clippy::blocks_in_if_conditions)]
            if precompile.feature.map_or(false, |ref feature_id| {
                self.feature_set.is_active(feature_id)
            }) {
                self.add_precompile(&precompile.program_id);
            }
        }
    }

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

    fn ensure_no_storage_rewards_pool(&mut self) {
        let purge_window_epoch = match self.cluster_type() {
            ClusterType::Development => false,
            // never do this for devnet; we're pristine here. :)
            ClusterType::Devnet => false,
            // schedule to remove at testnet/tds
            ClusterType::Testnet => self.epoch() == 93,
            // never do this for stable; we're pristine here. :)
            ClusterType::MainnetBeta => false,
        };

        if purge_window_epoch {
            for reward_pubkey in self.rewards_pool_pubkeys.iter() {
                if let Some(mut reward_account) = self.get_account_with_fixed_root(reward_pubkey) {
                    if reward_account.lamports() == u64::MAX {
                        reward_account.set_lamports(0);
                        self.store_account(reward_pubkey, &reward_account);
                        // Adjust capitalization.... it has been wrapping, reducing the real capitalization by 1-lamport
                        self.capitalization.fetch_add(1, Relaxed);
                        info!(
                            "purged rewards pool account: {}, new capitalization: {}",
                            reward_pubkey,
                            self.capitalization()
                        );
                    }
                };
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
            accounts_background_service::{AbsRequestHandler, SendDroppedBankCallback},
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
            system_instruction::{self, SystemError, MAX_PERMITTED_DATA_LENGTH},
            system_program,
            timing::duration_as_s,
            transaction::MAX_TX_ACCOUNT_LOCKS,
            transaction_context::InstructionContext,
        },
        solana_vote_program::{
            vote_instruction,
            vote_state::{
                self, BlockTimestamp, Vote, VoteInit, VoteState, VoteStateVersions,
                MAX_LOCKOUT_HISTORY,
            },
        },
        std::{
            result, str::FromStr, sync::atomic::Ordering::Release, thread::Builder, time::Duration,
        },
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
            min_rent_excempt_balance_for_sysvars(&bank, &[sysvar::slot_history::id()]);
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
            _first_instruction_account: usize,
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
        let mut vote_state = Some(VoteState::from(&vote_account).unwrap());
        for i in 0..MAX_LOCKOUT_HISTORY + 42 {
            if let Some(v) = vote_state.as_mut() {
                v.process_slot_vote_unchecked(i as u64)
            }
            let versioned = VoteStateVersions::Current(Box::new(vote_state.take().unwrap()));
            VoteState::to(&versioned, &mut vote_account).unwrap();
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
        let mut vote_state = Some(VoteState::from(&vote_account).unwrap());
        for i in 0..MAX_LOCKOUT_HISTORY + 42 {
            if let Some(v) = vote_state.as_mut() {
                v.process_slot_vote_unchecked(i as u64)
            }
            let versioned = VoteStateVersions::Current(Box::new(vote_state.take().unwrap()));
            VoteState::to(&versioned, &mut vote_account).unwrap();
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
        bank.clean_accounts(false, false, None);
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
        bank0.clean_accounts(false, false, None);
        assert_eq!(bank0.update_accounts_hash(), hash);

        assert_eq!(
            bank0.get_account(&keypair.pubkey()).unwrap().lamports(),
            amount
        );
        assert_eq!(bank1.get_account(&keypair.pubkey()), None);

        info!("bank1 purge");
        bank1.clean_accounts(false, false, None);

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
        bank1.clean_accounts(false, false, None);

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
        let key = Keypair::new();
        let new_balance = bank.deposit(&key.pubkey(), 10).unwrap();
        assert_eq!(new_balance, 10);
        assert_eq!(bank.get_balance(&key.pubkey()), 10);

        // Existing account
        let new_balance = bank.deposit(&key.pubkey(), 3).unwrap();
        assert_eq!(new_balance, 13);
        assert_eq!(bank.get_balance(&key.pubkey()), 13);
    }

    #[test]
    fn test_bank_withdraw() {
        let bank = create_simple_test_bank(100);

        // Test no account
        let key = Keypair::new();
        assert_eq!(
            bank.withdraw(&key.pubkey(), 10),
            Err(TransactionError::AccountNotFound)
        );

        bank.deposit(&key.pubkey(), 3).unwrap();
        assert_eq!(bank.get_balance(&key.pubkey()), 3);

        // Low balance
        assert_eq!(
            bank.withdraw(&key.pubkey(), 10),
            Err(TransactionError::InsufficientFundsForFee)
        );

        // Enough balance
        assert_eq!(bank.withdraw(&key.pubkey(), 2), Ok(()));
        assert_eq!(bank.get_balance(&key.pubkey()), 1);
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

        let key = Keypair::new();
        let tx = system_transaction::transfer(
            &mint_keypair,
            &key.pubkey(),
            arbitrary_transfer_amount,
            bank.last_blockhash(),
        );

        let initial_balance = bank.get_balance(&leader);
        assert_eq!(bank.process_transaction(&tx), Ok(()));
        assert_eq!(bank.get_balance(&key.pubkey()), arbitrary_transfer_amount);
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
        let mut tx =
            system_transaction::transfer(&mint_keypair, &key.pubkey(), 1, bank.last_blockhash());
        // Create a bogus instruction to system_program to cause an instruction error
        tx.message.instructions[0].data[0] = 40;

        bank.process_transaction(&tx)
            .expect_err("instruction error");
        assert_eq!(bank.get_balance(&key.pubkey()), arbitrary_transfer_amount); // no change
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

        let key = Keypair::new();
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
            true,
        );

        let (expected_fee_collected, expected_fee_burned) =
            genesis_config.fee_rate_governor.burn(expected_fee_paid);

        let mut bank = Bank::new_for_tests(&genesis_config);

        let capitalization = bank.capitalization();

        let tx = system_transaction::transfer(
            &mint_keypair,
            &key.pubkey(),
            arbitrary_transfer_amount,
            bank.last_blockhash(),
        );

        let initial_balance = bank.get_balance(&leader);
        assert_eq!(bank.process_transaction(&tx), Ok(()));
        assert_eq!(bank.get_balance(&key.pubkey()), arbitrary_transfer_amount);
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
        let mut tx =
            system_transaction::transfer(&mint_keypair, &key.pubkey(), 1, bank.last_blockhash());
        // Create a bogus instruction to system_program to cause an instruction error
        tx.message.instructions[0].data[0] = 40;

        bank.process_transaction(&tx)
            .expect_err("instruction error");
        assert_eq!(bank.get_balance(&key.pubkey()), arbitrary_transfer_amount); // no change
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
        let key = Keypair::new();
        let initial_mint_balance = bank.get_balance(&mint_keypair.pubkey());
        let tx = system_transaction::transfer(&mint_keypair, &key.pubkey(), 1, cheap_blockhash);
        assert_eq!(bank.process_transaction(&tx), Ok(()));
        assert_eq!(bank.get_balance(&key.pubkey()), 1);
        assert_eq!(
            bank.get_balance(&mint_keypair.pubkey()),
            initial_mint_balance - 1 - cheap_lamports_per_signature
        );

        // Send a transfer using expensive_blockhash
        let key = Keypair::new();
        let initial_mint_balance = bank.get_balance(&mint_keypair.pubkey());
        let tx = system_transaction::transfer(&mint_keypair, &key.pubkey(), 1, expensive_blockhash);
        assert_eq!(bank.process_transaction(&tx), Ok(()));
        assert_eq!(bank.get_balance(&key.pubkey()), 1);
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
        let key = Keypair::new();
        let initial_mint_balance = bank.get_balance(&mint_keypair.pubkey());
        let tx = system_transaction::transfer(&mint_keypair, &key.pubkey(), 1, cheap_blockhash);
        assert_eq!(bank.process_transaction(&tx), Ok(()));
        assert_eq!(bank.get_balance(&key.pubkey()), 1);
        let cheap_fee = Bank::calculate_fee(
            &SanitizedMessage::try_from(Message::new(&[], Some(&Pubkey::new_unique()))).unwrap(),
            cheap_lamports_per_signature,
            &FeeStructure::default(),
            true,
            true,
        );
        assert_eq!(
            bank.get_balance(&mint_keypair.pubkey()),
            initial_mint_balance - 1 - cheap_fee
        );

        // Send a transfer using expensive_blockhash
        let key = Keypair::new();
        let initial_mint_balance = bank.get_balance(&mint_keypair.pubkey());
        let tx = system_transaction::transfer(&mint_keypair, &key.pubkey(), 1, expensive_blockhash);
        assert_eq!(bank.process_transaction(&tx), Ok(()));
        assert_eq!(bank.get_balance(&key.pubkey()), 1);
        let expensive_fee = Bank::calculate_fee(
            &SanitizedMessage::try_from(Message::new(&[], Some(&Pubkey::new_unique()))).unwrap(),
            expensive_lamports_per_signature,
            &FeeStructure::default(),
            true,
            true,
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

        let key = Keypair::new();
        let tx1 = SanitizedTransaction::from_transaction_for_tests(system_transaction::transfer(
            &mint_keypair,
            &key.pubkey(),
            2,
            genesis_config.hash(),
        ));
        let tx2 = SanitizedTransaction::from_transaction_for_tests(system_transaction::transfer(
            &mint_keypair,
            &key.pubkey(),
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

        let key = Keypair::new();
        let tx1 = SanitizedTransaction::from_transaction_for_tests(system_transaction::transfer(
            &mint_keypair,
            &key.pubkey(),
            2,
            genesis_config.hash(),
        ));
        let tx2 = SanitizedTransaction::from_transaction_for_tests(system_transaction::transfer(
            &mint_keypair,
            &key.pubkey(),
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
                            true,
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
        assert!(bank.verify_snapshot_bank(true, false, None));

        // tamper the bank after freeze!
        bank.increment_signature_count(1);
        assert!(!bank.verify_snapshot_bank(true, false, None));
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
                    old + min_rent_excempt_balance_for_sysvars(&bank1, &[sysvar::clock::id()]),
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
        let key = Keypair::new();

        let mut transfer_instruction =
            system_instruction::transfer(&mint_keypair.pubkey(), &key.pubkey(), 0);
        transfer_instruction.accounts[0].is_signer = false;
        let message = Message::new(&[transfer_instruction], None);
        let tx = Transaction::new(&[&Keypair::new(); 0], message, bank.last_blockhash());

        assert_eq!(
            bank.process_transaction(&tx),
            Err(TransactionError::SanitizeFailure)
        );
        assert_eq!(bank.get_balance(&key.pubkey()), 0);
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
            _first_instruction_account: usize,
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
            _first_instruction_account: usize,
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
            _first_instruction_account: usize,
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
                    std::thread::sleep(Duration::new(5, 0));
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
            _first_instruction_account: usize,
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
            _first_instruction_account: usize,
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

        while tx.message.account_keys.len() <= MAX_TX_ACCOUNT_LOCKS {
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
        _first_instruction_account: usize,
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
                    "9tLrxkBoNE7zEUZ2g72ZwE4fTfhUQnhC8A4Xt4EmYhP1"
                );
            }
            if bank.slot == 32 {
                assert_eq!(
                    bank.hash().to_string(),
                    "AxphC8xDj9gmFosor5gyiovNvPVMydJCFRUTxn2wFiQf"
                );
            }
            if bank.slot == 64 {
                assert_eq!(
                    bank.hash().to_string(),
                    "4vZCSbBuL8xjE43rCy9Cm3dCh1BMj45heMiMb6n6qgzA"
                );
            }
            if bank.slot == 128 {
                assert_eq!(
                    bank.hash().to_string(),
                    "46LUpeBdJuisnfwgYisvh4x7jnxzBaLfHF614GtcTs59"
                );
                break;
            }
            bank = Arc::new(new_from_parent(&bank));
        }
    }

    #[test]
    fn test_same_program_id_uses_unqiue_executable_accounts() {
        fn nested_processor(
            _first_instruction_account: usize,
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
        bank1.flush_accounts_cache_slot();

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
        bank2.clean_accounts(false, false, None);

        let mut bank3 = Arc::new(Bank::new_from_parent(&bank2, &Pubkey::default(), 3));
        bank3.deposit(&pubkey1, some_lamports + 1).unwrap();
        goto_end_of_slot(Arc::<Bank>::get_mut(&mut bank3).unwrap());
        bank3.freeze();
        bank3.squash();
        bank3.force_flush_accounts_cache();

        bank3.clean_accounts(false, false, None);
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
        bank2.clean_accounts(false, false, None);

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
        assert_eq!(alive_counts, vec![9, 1, 7]);
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
        bank.clean_accounts(false, false, None);
        let force_to_return_alive_account = 0;
        assert_eq!(
            bank.process_stale_slot_with_budget(22, force_to_return_alive_account),
            22
        );

        let consumed_budgets: usize = (0..3)
            .map(|_| bank.process_stale_slot_with_budget(0, force_to_return_alive_account))
            .sum();
        // consumed_budgets represents the count of alive accounts in the three slots 0,1,2
        assert_eq!(consumed_budgets, 10);
    }

    #[test]
    fn test_add_builtin_no_overwrite() {
        #[allow(clippy::unnecessary_wraps)]
        fn mock_ix_processor(
            _first_instruction_account: usize,
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
            _first_instruction_account: usize,
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
