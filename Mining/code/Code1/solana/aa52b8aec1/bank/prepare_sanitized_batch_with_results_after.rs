    pub fn prepare_sanitized_batch_with_results<'a, 'b>(
        &'a self,
        transactions: &'b [SanitizedTransaction],
        transaction_results: impl Iterator<Item = &'b Result<()>>,
    ) -> TransactionBatch<'a, 'b> {
        // this lock_results could be: Ok, AccountInUse, WouldExceedBlockMaxLimit or WouldExceedAccountMaxLimit
        let tx_account_lock_limit = self.get_transaction_account_lock_limit();
        let lock_results = self.rc.accounts.lock_accounts_with_results(
            transactions.iter(),
            transaction_results,
            tx_account_lock_limit,
        );
        TransactionBatch::new(lock_results, self, Cow::Borrowed(transactions))
    }

    /// Prepare a transaction batch without locking accounts for transaction simulation.
    pub(crate) fn prepare_simulation_batch(
        &self,
        transaction: SanitizedTransaction,
    ) -> TransactionBatch<'_, '_> {
        let tx_account_lock_limit = self.get_transaction_account_lock_limit();
        let lock_result = transaction
            .get_account_locks(tx_account_lock_limit)
            .map(|_| ());
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
        if self
            .feature_set
            .is_active(&feature_set::cap_accounts_data_allocations_per_transaction::id())
        {
            transaction_context.enable_cap_accounts_data_allocations_per_transaction();
        }

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
            &self.sysvar_cache.read().unwrap(),
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

        let inner_instructions = if enable_cpi_recording {
            Some(inner_instructions_list_from_instruction_trace(
                &transaction_context,
            ))
        } else {
            None
        };

        let ExecutionRecord {
            accounts,
            mut return_data,
            touched_account_count,
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
            saturating_add_assign!(timings.details.changed_account_count, touched_account_count);
            accounts_data_len_delta = status.as_ref().map_or(0, |_| accounts_resize_delta);
        }

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
                    let compute_budget =
                        if let Some(compute_budget) = self.runtime_config.compute_budget {
                            compute_budget
                        } else {
                            let mut compute_budget =
                                ComputeBudget::new(compute_budget::MAX_COMPUTE_UNIT_LIMIT as u64);

                            let mut compute_budget_process_transaction_time =
                                Measure::start("compute_budget_process_transaction_time");
                            let process_transaction_result = compute_budget.process_instructions(
                                tx.message().program_instructions_iter(),
                                true,
                                !self
                                    .feature_set
                                    .is_active(&remove_deprecated_request_unit_ix::id()),
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
        use_default_units_per_instruction: bool,
        support_request_units_deprecated: bool,
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
                support_request_units_deprecated,
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
                        .is_active(&use_default_units_in_fee_calculation::id()),
                    !self
                        .feature_set
                        .is_active(&remove_deprecated_request_unit_ix::id()),
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

    /// If we are skipping rewrites for bank hash, then we don't want to
    ///  allow accounts hash calculation to rehash anything.
    ///  We should use whatever hash found for each account as-is.
    pub fn bank_enable_rehashing_on_accounts_hash(&self) -> bool {
        true // this will be governed by a feature later
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
