use {
    crate::{
        accounts_data_meter::AccountsDataMeter, ic_logger_msg, ic_msg,
        instruction_recorder::InstructionRecorder, log_collector::LogCollector,
        native_loader::NativeLoader, pre_account::PreAccount, timings::ExecuteDetailsTimings,
    },
    solana_sdk::{
        account::{AccountSharedData, ReadableAccount},
        account_utils::StateMut,
        bpf_loader_upgradeable::{self, UpgradeableLoaderState},
        compute_budget::ComputeBudget,
        feature_set::{
            do_support_realloc, neon_evm_compute_budget, reject_empty_instruction_without_program,
            remove_native_loader, requestable_heap_size, tx_wide_compute_cap, FeatureSet,
        },
        hash::Hash,
        instruction::{AccountMeta, CompiledInstruction, Instruction, InstructionError},
        keyed_account::{create_keyed_accounts_unified, keyed_account_at_index, KeyedAccount},
        native_loader,
        pubkey::Pubkey,
        rent::Rent,
        sysvar::Sysvar,
        transaction_context::{InstructionAccount, TransactionAccount, TransactionContext},
    },
    std::{cell::RefCell, collections::HashMap, fmt::Debug, rc::Rc, sync::Arc},
};

pub type ProcessInstructionWithContext =
    fn(usize, &[u8], &mut InvokeContext) -> Result<(), InstructionError>;

#[derive(Debug, PartialEq)]
pub struct ProcessInstructionResult {
    pub compute_units_consumed: u64,
    pub result: Result<(), InstructionError>,
}

#[derive(Clone)]
pub struct BuiltinProgram {
    pub program_id: Pubkey,
    pub process_instruction: ProcessInstructionWithContext,
}

impl std::fmt::Debug for BuiltinProgram {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        // These are just type aliases for work around of Debug-ing above pointers
        type ErasedProcessInstructionWithContext = fn(
            usize,
            &'static [u8],
            &'static mut InvokeContext<'static>,
        ) -> Result<(), InstructionError>;

        // rustc doesn't compile due to bug without this work around
        // https://github.com/rust-lang/rust/issues/50280
        // https://users.rust-lang.org/t/display-function-pointer/17073/2
        let erased_instruction: ErasedProcessInstructionWithContext = self.process_instruction;
        write!(f, "{}: {:p}", self.program_id, erased_instruction)
    }
}

/// Program executor
pub trait Executor: Debug + Send + Sync {
    /// Execute the program
    fn execute<'a, 'b>(
        &self,
        first_instruction_account: usize,
        instruction_data: &[u8],
        invoke_context: &'a mut InvokeContext<'b>,
        use_jit: bool,
    ) -> Result<(), InstructionError>;
}

#[derive(Default)]
pub struct Executors {
    pub executors: HashMap<Pubkey, Arc<dyn Executor>>,
    pub is_dirty: bool,
}
impl Executors {
    pub fn insert(&mut self, key: Pubkey, executor: Arc<dyn Executor>) {
        let _ = self.executors.insert(key, executor);
        self.is_dirty = true;
    }
    pub fn get(&self, key: &Pubkey) -> Option<Arc<dyn Executor>> {
        self.executors.get(key).cloned()
    }
}

/// Compute meter
pub struct ComputeMeter {
    remaining: u64,
}
impl ComputeMeter {
    /// Consume compute units
    pub fn consume(&mut self, amount: u64) -> Result<(), InstructionError> {
        let exceeded = self.remaining < amount;
        self.remaining = self.remaining.saturating_sub(amount);
        if exceeded {
            return Err(InstructionError::ComputationalBudgetExceeded);
        }
        Ok(())
    }
    /// Get the number of remaining compute units
    pub fn get_remaining(&self) -> u64 {
        self.remaining
    }
    /// Set compute units
    ///
    /// Only use for tests and benchmarks
    pub fn mock_set_remaining(&mut self, remaining: u64) {
        self.remaining = remaining;
    }
    /// Construct a new one with the given remaining units
    pub fn new_ref(remaining: u64) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self { remaining }))
    }
}

pub struct StackFrame<'a> {
    pub number_of_program_accounts: usize,
    pub keyed_accounts: Vec<KeyedAccount<'a>>,
    pub keyed_accounts_range: std::ops::Range<usize>,
}

impl<'a> StackFrame<'a> {
    pub fn new(number_of_program_accounts: usize, keyed_accounts: Vec<KeyedAccount<'a>>) -> Self {
        let keyed_accounts_range = std::ops::Range {
            start: 0,
            end: keyed_accounts.len(),
        };
        Self {
            number_of_program_accounts,
            keyed_accounts,
            keyed_accounts_range,
        }
    }

    pub fn program_id(&self) -> Option<&Pubkey> {
        self.keyed_accounts
            .get(self.number_of_program_accounts.saturating_sub(1))
            .map(|keyed_account| keyed_account.unsigned_key())
    }
}

pub struct InvokeContext<'a> {
    pub transaction_context: &'a TransactionContext,
    pub return_data: (Pubkey, Vec<u8>),
    invoke_stack: Vec<StackFrame<'a>>,
    rent: Rent,
    pre_accounts: Vec<PreAccount>,
    builtin_programs: &'a [BuiltinProgram],
    pub sysvars: &'a [(Pubkey, Vec<u8>)],
    log_collector: Option<Rc<RefCell<LogCollector>>>,
    compute_budget: ComputeBudget,
    current_compute_budget: ComputeBudget,
    compute_meter: Rc<RefCell<ComputeMeter>>,
    accounts_data_meter: AccountsDataMeter,
    executors: Rc<RefCell<Executors>>,
    pub instruction_recorder: Option<Rc<RefCell<InstructionRecorder>>>,
    pub feature_set: Arc<FeatureSet>,
    pub timings: ExecuteDetailsTimings,
    pub blockhash: Hash,
    pub lamports_per_signature: u64,
}

impl<'a> InvokeContext<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        transaction_context: &'a TransactionContext,
        rent: Rent,
        builtin_programs: &'a [BuiltinProgram],
        sysvars: &'a [(Pubkey, Vec<u8>)],
        log_collector: Option<Rc<RefCell<LogCollector>>>,
        compute_budget: ComputeBudget,
        executors: Rc<RefCell<Executors>>,
        instruction_recorder: Option<Rc<RefCell<InstructionRecorder>>>,
        feature_set: Arc<FeatureSet>,
        blockhash: Hash,
        lamports_per_signature: u64,
        current_accounts_data_len: u64,
    ) -> Self {
        Self {
            transaction_context,
            return_data: (Pubkey::default(), Vec::new()),
            invoke_stack: Vec::with_capacity(compute_budget.max_invoke_depth),
            rent,
            pre_accounts: Vec::new(),
            builtin_programs,
            sysvars,
            log_collector,
            current_compute_budget: compute_budget,
            compute_budget,
            compute_meter: ComputeMeter::new_ref(compute_budget.max_units),
            accounts_data_meter: AccountsDataMeter::new(current_accounts_data_len),
            executors,
            instruction_recorder,
            feature_set,
            timings: ExecuteDetailsTimings::default(),
            blockhash,
            lamports_per_signature,
        }
    }

    pub fn new_mock(
        transaction_context: &'a TransactionContext,
        builtin_programs: &'a [BuiltinProgram],
    ) -> Self {
        Self::new(
            transaction_context,
            Rent::default(),
            builtin_programs,
            &[],
            Some(LogCollector::new_ref()),
            ComputeBudget::default(),
            Rc::new(RefCell::new(Executors::default())),
            None,
            Arc::new(FeatureSet::all_enabled()),
            Hash::default(),
            0,
            0,
        )
    }

    /// Push a stack frame onto the invocation stack
    pub fn push(
        &mut self,
        instruction_accounts: &[InstructionAccount],
        program_indices: &[usize],
    ) -> Result<(), InstructionError> {
        if self.invoke_stack.len() > self.compute_budget.max_invoke_depth {
            return Err(InstructionError::CallDepth);
        }

        let program_id = program_indices.last().map(|account_index| {
            self.transaction_context
                .get_key_of_account_at_index(*account_index)
        });
        if program_id.is_none()
            && self
                .feature_set
                .is_active(&reject_empty_instruction_without_program::id())
        {
            return Err(InstructionError::UnsupportedProgramId);
        }
        if self.invoke_stack.is_empty() {
            let mut compute_budget = self.compute_budget;
            if !self.feature_set.is_active(&tx_wide_compute_cap::id())
                && self.feature_set.is_active(&neon_evm_compute_budget::id())
                && program_id == Some(&crate::neon_evm_program::id())
            {
                // Bump the compute budget for neon_evm
                compute_budget.max_units = compute_budget.max_units.max(500_000);
            }
            if !self.feature_set.is_active(&requestable_heap_size::id())
                && self.feature_set.is_active(&neon_evm_compute_budget::id())
                && program_id == Some(&crate::neon_evm_program::id())
            {
                // Bump the compute budget for neon_evm
                compute_budget.heap_size = Some(256_usize.saturating_mul(1024));
            }
            self.current_compute_budget = compute_budget;

            if !self.feature_set.is_active(&tx_wide_compute_cap::id()) {
                self.compute_meter = ComputeMeter::new_ref(self.current_compute_budget.max_units);
            }

            self.pre_accounts = Vec::with_capacity(instruction_accounts.len());
            let mut work = |_index_in_instruction: usize,
                            instruction_account: &InstructionAccount| {
                if instruction_account.index_in_transaction
                    < self.transaction_context.get_number_of_accounts()
                {
                    let account = self
                        .transaction_context
                        .get_account_at_index(instruction_account.index_in_transaction)
                        .borrow()
                        .clone();
                    self.pre_accounts.push(PreAccount::new(
                        self.transaction_context
                            .get_key_of_account_at_index(instruction_account.index_in_transaction),
                        account,
                    ));
                    return Ok(());
                }
                Err(InstructionError::MissingAccount)
            };
            visit_each_account_once(instruction_accounts, &mut work)?;
        } else {
            let contains = self
                .invoke_stack
                .iter()
                .any(|frame| frame.program_id() == program_id);
            let is_last = if let Some(last_frame) = self.invoke_stack.last() {
                last_frame.program_id() == program_id
            } else {
                false
            };
            if contains && !is_last {
                // Reentrancy not allowed unless caller is calling itself
                return Err(InstructionError::ReentrancyNotAllowed);
            }
        }

        // Create the KeyedAccounts that will be passed to the program
        let keyed_accounts = program_indices
            .iter()
            .map(|account_index| {
                (
                    false,
                    false,
                    self.transaction_context
                        .get_key_of_account_at_index(*account_index),
                    self.transaction_context
                        .get_account_at_index(*account_index),
                )
            })
            .chain(instruction_accounts.iter().map(|instruction_account| {
                (
                    instruction_account.is_signer,
                    instruction_account.is_writable,
                    self.transaction_context
                        .get_key_of_account_at_index(instruction_account.index_in_transaction),
                    self.transaction_context
                        .get_account_at_index(instruction_account.index_in_transaction),
                )
            }))
            .collect::<Vec<_>>();

        self.invoke_stack.push(StackFrame::new(
            program_indices.len(),
            create_keyed_accounts_unified(keyed_accounts.as_slice()),
        ));
        Ok(())
    }

    /// Pop a stack frame from the invocation stack
    pub fn pop(&mut self) {
        self.invoke_stack.pop();
    }

    /// Current depth of the invocation stack
    pub fn invoke_depth(&self) -> usize {
        self.invoke_stack.len()
    }

    /// Verify the results of an instruction
    fn verify(
        &mut self,
        instruction_accounts: &[InstructionAccount],
        program_indices: &[usize],
    ) -> Result<(), InstructionError> {
        let program_id = self
            .invoke_stack
            .last()
            .and_then(|frame| frame.program_id())
            .ok_or(InstructionError::CallDepth)?;
        let do_support_realloc = self.feature_set.is_active(&do_support_realloc::id());

        // Verify all executable accounts have zero outstanding refs
        for account_index in program_indices.iter() {
            self.transaction_context
                .get_account_at_index(*account_index)
                .try_borrow_mut()
                .map_err(|_| InstructionError::AccountBorrowOutstanding)?;
        }

        // Verify the per-account instruction results
        let (mut pre_sum, mut post_sum) = (0_u128, 0_u128);
        let mut pre_account_index = 0;
        let mut work = |_index_in_instruction: usize, instruction_account: &InstructionAccount| {
            {
                // Verify account has no outstanding references
                let _ = self
                    .transaction_context
                    .get_account_at_index(instruction_account.index_in_transaction)
                    .try_borrow_mut()
                    .map_err(|_| InstructionError::AccountBorrowOutstanding)?;
            }
            let pre_account = &self.pre_accounts[pre_account_index];
            pre_account_index = pre_account_index.saturating_add(1);
            let account = self
                .transaction_context
                .get_account_at_index(instruction_account.index_in_transaction)
                .borrow();
            pre_account
                .verify(
                    program_id,
                    instruction_account.is_writable,
                    &self.rent,
                    &account,
                    &mut self.timings,
                    true,
                    do_support_realloc,
                )
                .map_err(|err| {
                    ic_logger_msg!(
                        self.log_collector,
                        "failed to verify account {}: {}",
                        pre_account.key(),
                        err
                    );
                    err
                })?;
            pre_sum = pre_sum
                .checked_add(u128::from(pre_account.lamports()))
                .ok_or(InstructionError::UnbalancedInstruction)?;
            post_sum = post_sum
                .checked_add(u128::from(account.lamports()))
                .ok_or(InstructionError::UnbalancedInstruction)?;
            Ok(())
        };
        visit_each_account_once(instruction_accounts, &mut work)?;

        // Verify that the total sum of all the lamports did not change
        if pre_sum != post_sum {
            return Err(InstructionError::UnbalancedInstruction);
        }
        Ok(())
    }

    /// Verify and update PreAccount state based on program execution
    fn verify_and_update(
        &mut self,
        instruction_accounts: &[InstructionAccount],
        caller_write_privileges: Option<&[bool]>,
    ) -> Result<(), InstructionError> {
        let do_support_realloc = self.feature_set.is_active(&do_support_realloc::id());
        let program_id = self
            .invoke_stack
            .last()
            .and_then(|frame| frame.program_id())
            .ok_or(InstructionError::CallDepth)?;
        let rent = &self.rent;
        let log_collector = &self.log_collector;
        let transaction_context = self.transaction_context;
        let pre_accounts = &mut self.pre_accounts;
        let timings = &mut self.timings;

        // Verify the per-account instruction results
        let (mut pre_sum, mut post_sum) = (0_u128, 0_u128);
        let mut work = |index_in_instruction: usize, instruction_account: &InstructionAccount| {
            if instruction_account.index_in_transaction
                < transaction_context.get_number_of_accounts()
            {
                let key = transaction_context
                    .get_key_of_account_at_index(instruction_account.index_in_transaction);
                let account = transaction_context
                    .get_account_at_index(instruction_account.index_in_transaction);
                let is_writable = if let Some(caller_write_privileges) = caller_write_privileges {
                    caller_write_privileges[index_in_instruction]
                } else {
                    instruction_account.is_writable
                };
                // Find the matching PreAccount
                for pre_account in pre_accounts.iter_mut() {
                    if key == pre_account.key() {
                        {
                            // Verify account has no outstanding references
                            let _ = account
                                .try_borrow_mut()
                                .map_err(|_| InstructionError::AccountBorrowOutstanding)?;
                        }
                        let account = account.borrow();
                        pre_account
                            .verify(
                                program_id,
                                is_writable,
                                rent,
                                &account,
                                timings,
                                false,
                                do_support_realloc,
                            )
                            .map_err(|err| {
                                ic_logger_msg!(
                                    log_collector,
                                    "failed to verify account {}: {}",
                                    key,
                                    err
                                );
                                err
                            })?;
                        pre_sum = pre_sum
                            .checked_add(u128::from(pre_account.lamports()))
                            .ok_or(InstructionError::UnbalancedInstruction)?;
                        post_sum = post_sum
                            .checked_add(u128::from(account.lamports()))
                            .ok_or(InstructionError::UnbalancedInstruction)?;
                        if is_writable && !pre_account.executable() {
                            pre_account.update(account.clone());
                        }
                        return Ok(());
                    }
                }
            }
            Err(InstructionError::MissingAccount)
        };
        visit_each_account_once(instruction_accounts, &mut work)?;

        // Verify that the total sum of all the lamports did not change
        if pre_sum != post_sum {
            return Err(InstructionError::UnbalancedInstruction);
        }
        Ok(())
    }

    /// Entrypoint for a cross-program invocation from a builtin program
    pub fn native_invoke(
        &mut self,
        instruction: Instruction,
        signers: &[Pubkey],
    ) -> Result<(), InstructionError> {
        let (instruction_accounts, caller_write_privileges, program_indices) =
            self.prepare_instruction(&instruction, signers)?;
        let mut prev_account_sizes = Vec::with_capacity(instruction_accounts.len());
        for instruction_account in instruction_accounts.iter() {
            let account_length = self
                .transaction_context
                .get_account_at_index(instruction_account.index_in_transaction)
                .borrow()
                .data()
                .len();
            prev_account_sizes.push((instruction_account.index_in_transaction, account_length));
        }

        self.process_instruction(
            &instruction.data,
            &instruction_accounts,
            Some(&caller_write_privileges),
            &program_indices,
        )
        .result?;

        // Verify the called program has not misbehaved
        let do_support_realloc = self.feature_set.is_active(&do_support_realloc::id());
        for (account_index, prev_size) in prev_account_sizes.into_iter() {
            if !do_support_realloc
                && prev_size
                    != self
                        .transaction_context
                        .get_account_at_index(account_index)
                        .borrow()
                        .data()
                        .len()
                && prev_size != 0
            {
                // Only support for `CreateAccount` at this time.
                // Need a way to limit total realloc size across multiple CPI calls
                ic_msg!(
                    self,
                    "Inner instructions do not support realloc, only SystemProgram::CreateAccount",
                );
                return Err(InstructionError::InvalidRealloc);
            }
        }

        Ok(())
    }

    /// Helper to prepare for process_instruction()
    #[allow(clippy::type_complexity)]
    pub fn prepare_instruction(
        &mut self,
        instruction: &Instruction,
        signers: &[Pubkey],
    ) -> Result<(Vec<InstructionAccount>, Vec<bool>, Vec<usize>), InstructionError> {
        // Finds the index of each account in the instruction by its pubkey.
        // Then normalizes / unifies the privileges of duplicate accounts.
        // Note: This works like visit_each_account_once() and is an O(n^2) algorithm too.
        let caller_keyed_accounts = self.get_keyed_accounts()?;
        let mut deduplicated_instruction_accounts: Vec<InstructionAccount> = Vec::new();
        let mut duplicate_indicies = Vec::with_capacity(instruction.accounts.len());
        for account_meta in instruction.accounts.iter() {
            let index_in_transaction = self
                .transaction_context
                .find_index_of_account(&account_meta.pubkey)
                .ok_or_else(|| {
                    ic_msg!(
                        self,
                        "Instruction references an unknown account {}",
                        account_meta.pubkey,
                    );
                    InstructionError::MissingAccount
                })?;
            if let Some(duplicate_index) =
                deduplicated_instruction_accounts
                    .iter()
                    .position(|instruction_account| {
                        instruction_account.index_in_transaction == index_in_transaction
                    })
            {
                duplicate_indicies.push(duplicate_index);
                let instruction_account = &mut deduplicated_instruction_accounts[duplicate_index];
                instruction_account.is_signer |= account_meta.is_signer;
                instruction_account.is_writable |= account_meta.is_writable;
            } else {
                let index_in_caller = caller_keyed_accounts
                    .iter()
                    .position(|keyed_account| *keyed_account.unsigned_key() == account_meta.pubkey)
                    .ok_or_else(|| {
                        ic_msg!(
                            self,
                            "Instruction references an unknown account {}",
                            account_meta.pubkey,
                        );
                        InstructionError::MissingAccount
                    })?;
                duplicate_indicies.push(deduplicated_instruction_accounts.len());
                deduplicated_instruction_accounts.push(InstructionAccount {
                    index_in_transaction,
                    index_in_caller,
                    is_signer: account_meta.is_signer,
                    is_writable: account_meta.is_writable,
                });
            }
        }
        let instruction_accounts: Vec<InstructionAccount> = duplicate_indicies
            .into_iter()
            .map(|duplicate_index| deduplicated_instruction_accounts[duplicate_index].clone())
            .collect();

        // Check for privilege escalation
        let caller_write_privileges = instruction_accounts
            .iter()
            .map(|instruction_account| {
                let keyed_account = &caller_keyed_accounts[instruction_account.index_in_caller];

                // Readonly in caller cannot become writable in callee
                if instruction_account.is_writable && !keyed_account.is_writable() {
                    ic_msg!(
                        self,
                        "{}'s writable privilege escalated",
                        keyed_account.unsigned_key(),
                    );
                    return Err(InstructionError::PrivilegeEscalation);
                }

                // To be signed in the callee,
                // it must be either signed in the caller or by the program
                if instruction_account.is_signer
                    && !(keyed_account.signer_key().is_some()
                        || signers.contains(keyed_account.unsigned_key()))
                {
                    ic_msg!(
                        self,
                        "{}'s signer privilege escalated",
                        keyed_account.unsigned_key()
                    );
                    return Err(InstructionError::PrivilegeEscalation);
                }

                Ok(keyed_account.is_writable())
            })
            .collect::<Result<Vec<bool>, InstructionError>>()?;

        // Find and validate executables / program accounts
        let callee_program_id = instruction.program_id;
        let program_account_index = caller_keyed_accounts
            .iter()
            .find(|keyed_account| &callee_program_id == keyed_account.unsigned_key())
            .and_then(|_keyed_account| {
                self.transaction_context
                    .find_index_of_program_account(&callee_program_id)
            })
            .ok_or_else(|| {
                ic_msg!(self, "Unknown program {}", callee_program_id);
                InstructionError::MissingAccount
            })?;
        let program_account = self
            .transaction_context
            .get_account_at_index(program_account_index)
            .borrow();
        if !program_account.executable() {
            ic_msg!(self, "Account {} is not executable", callee_program_id);
            return Err(InstructionError::AccountNotExecutable);
        }
        let mut program_indices = vec![];
        if program_account.owner() == &bpf_loader_upgradeable::id() {
            if let UpgradeableLoaderState::Program {
                programdata_address,
            } = program_account.state()?
            {
                if let Some(programdata_account_index) = self
                    .transaction_context
                    .find_index_of_program_account(&programdata_address)
                {
                    program_indices.push(programdata_account_index);
                } else {
                    ic_msg!(
                        self,
                        "Unknown upgradeable programdata account {}",
                        programdata_address,
                    );
                    return Err(InstructionError::MissingAccount);
                }
            } else {
                ic_msg!(
                    self,
                    "Invalid upgradeable program account {}",
                    callee_program_id,
                );
                return Err(InstructionError::MissingAccount);
            }
        }
        program_indices.push(program_account_index);

        Ok((
            instruction_accounts,
            caller_write_privileges,
            program_indices,
        ))
    }

    /// Processes a cross-program instruction and returns how many compute units were used
    pub fn process_instruction(
        &mut self,
        instruction_data: &[u8],
        instruction_accounts: &[InstructionAccount],
        caller_write_privileges: Option<&[bool]>,
        program_indices: &[usize],
    ) -> ProcessInstructionResult {
        let program_id = program_indices
            .last()
            .map(|index| *self.transaction_context.get_key_of_account_at_index(*index))
            .unwrap_or_else(native_loader::id);

        let is_lowest_invocation_level = self.invoke_stack.is_empty();
        if is_lowest_invocation_level {
            if let Some(instruction_recorder) = &self.instruction_recorder {
                instruction_recorder.borrow_mut().begin_next_recording();
            }
        } else {
            // Verify the calling program hasn't misbehaved
            let result = self.verify_and_update(instruction_accounts, caller_write_privileges);
            if result.is_err() {
                return ProcessInstructionResult {
                    compute_units_consumed: 0,
                    result,
                };
            }
            // Record instruction
            if let Some(instruction_recorder) = &self.instruction_recorder {
                let compiled_instruction = CompiledInstruction {
                    program_id_index: self
                        .transaction_context
                        .find_index_of_account(&program_id)
                        .unwrap_or(0) as u8,
                    data: instruction_data.to_vec(),
                    accounts: instruction_accounts
                        .iter()
                        .map(|instruction_account| instruction_account.index_in_transaction as u8)
                        .collect(),
                };
                instruction_recorder
                    .borrow_mut()
                    .record_compiled_instruction(compiled_instruction);
            }
        }

        let mut compute_units_consumed = 0;
        let result = self
            .push(instruction_accounts, program_indices)
            .and_then(|_| {
                self.return_data = (program_id, Vec::new());
                let pre_remaining_units = self.compute_meter.borrow().get_remaining();
                let execution_result = self.process_executable_chain(instruction_data);
                let post_remaining_units = self.compute_meter.borrow().get_remaining();
                compute_units_consumed = pre_remaining_units.saturating_sub(post_remaining_units);
                execution_result?;

                // Verify the called program has not misbehaved
                if is_lowest_invocation_level {
                    self.verify(instruction_accounts, program_indices)
                } else {
                    self.verify_and_update(instruction_accounts, None)
                }
            });

        // Pop the invoke_stack to restore previous state
        self.pop();
        ProcessInstructionResult {
            compute_units_consumed,
            result,
        }
    }

    /// Calls the instruction's program entrypoint method
    fn process_executable_chain(
        &mut self,
        instruction_data: &[u8],
    ) -> Result<(), InstructionError> {
        let keyed_accounts = self.get_keyed_accounts()?;
        let root_account = keyed_account_at_index(keyed_accounts, 0)
            .map_err(|_| InstructionError::UnsupportedProgramId)?;
        let root_id = root_account.unsigned_key();
        let owner_id = &root_account.owner()?;
        if solana_sdk::native_loader::check_id(owner_id) {
            for entry in self.builtin_programs {
                if entry.program_id == *root_id {
                    // Call the builtin program
                    return (entry.process_instruction)(
                        1, // root_id to be skipped
                        instruction_data,
                        self,
                    );
                }
            }
            if !self.feature_set.is_active(&remove_native_loader::id()) {
                let native_loader = NativeLoader::default();
                // Call the program via the native loader
                return native_loader.process_instruction(0, instruction_data, self);
            }
        } else {
            for entry in self.builtin_programs {
                if entry.program_id == *owner_id {
                    // Call the program via a builtin loader
                    return (entry.process_instruction)(
                        0, // no root_id was provided
                        instruction_data,
                        self,
                    );
                }
            }
        }
        Err(InstructionError::UnsupportedProgramId)
    }

    /// Get the program ID of the currently executing program
    pub fn get_caller(&self) -> Result<&Pubkey, InstructionError> {
        self.invoke_stack
            .last()
            .and_then(|frame| frame.program_id())
            .ok_or(InstructionError::CallDepth)
    }

    /// Get the owner of the currently executing program
    pub fn get_loader(&self) -> Result<Pubkey, InstructionError> {
        let frame = self
            .invoke_stack
            .last()
            .ok_or(InstructionError::CallDepth)?;
        let first_instruction_account = frame
            .number_of_program_accounts
            .checked_sub(1)
            .ok_or(InstructionError::CallDepth)?;
        frame.keyed_accounts[first_instruction_account].owner()
    }

    /// Removes the first keyed account
    #[deprecated(
        since = "1.9.0",
        note = "To be removed together with remove_native_loader"
    )]
    pub fn remove_first_keyed_account(&mut self) -> Result<(), InstructionError> {
        if !self.feature_set.is_active(&remove_native_loader::id()) {
            let stack_frame = &mut self
                .invoke_stack
                .last_mut()
                .ok_or(InstructionError::CallDepth)?;
            stack_frame.keyed_accounts_range.start =
                stack_frame.keyed_accounts_range.start.saturating_add(1);
        }
        Ok(())
    }

    /// Get the list of keyed accounts including the chain of program accounts
    pub fn get_keyed_accounts(&self) -> Result<&[KeyedAccount], InstructionError> {
        self.invoke_stack
            .last()
            .map(|frame| &frame.keyed_accounts[frame.keyed_accounts_range.clone()])
            .ok_or(InstructionError::CallDepth)
    }

    /// Get the list of keyed accounts without the chain of program accounts
    ///
    /// Note: This only contains the `KeyedAccount`s passed by the caller.
    pub fn get_instruction_keyed_accounts(&self) -> Result<&[KeyedAccount], InstructionError> {
        let frame = self
            .invoke_stack
            .last()
            .ok_or(InstructionError::CallDepth)?;
        Ok(&frame.keyed_accounts[frame.number_of_program_accounts..])
    }

    /// Get this invocation's LogCollector
    pub fn get_log_collector(&self) -> Option<Rc<RefCell<LogCollector>>> {
        self.log_collector.clone()
    }

    /// Get this invocation's ComputeMeter
    pub fn get_compute_meter(&self) -> Rc<RefCell<ComputeMeter>> {
        self.compute_meter.clone()
    }

    /// Get this invocation's AccountsDataMeter
    pub fn get_accounts_data_meter(&self) -> &AccountsDataMeter {
        &self.accounts_data_meter
    }

    /// Loaders may need to do work in order to execute a program. Cache
    /// the work that can be re-used across executions
    pub fn add_executor(&self, pubkey: &Pubkey, executor: Arc<dyn Executor>) {
        self.executors.borrow_mut().insert(*pubkey, executor);
    }

    /// Get the completed loader work that can be re-used across execution
    pub fn get_executor(&self, pubkey: &Pubkey) -> Option<Arc<dyn Executor>> {
        self.executors.borrow().get(pubkey)
    }

    /// Get this invocation's compute budget
    pub fn get_compute_budget(&self) -> &ComputeBudget {
        &self.current_compute_budget
    }

    /// Get the value of a sysvar by its id
    pub fn get_sysvar<T: Sysvar>(&self, id: &Pubkey) -> Result<T, InstructionError> {
        self.sysvars
            .iter()
            .find_map(|(key, data)| {
                if id == key {
                    bincode::deserialize(data).ok()
                } else {
                    None
                }
            })
            .ok_or_else(|| {
                ic_msg!(self, "Unable to get sysvar {}", id);
                InstructionError::UnsupportedSysvar
            })
    }
}

pub struct MockInvokeContextPreparation {
    pub transaction_accounts: Vec<TransactionAccount>,
    pub instruction_accounts: Vec<InstructionAccount>,
}

pub fn prepare_mock_invoke_context(
    transaction_accounts: Vec<TransactionAccount>,
    instruction_accounts: Vec<AccountMeta>,
    program_indices: &[usize],
) -> MockInvokeContextPreparation {
    let instruction_accounts = instruction_accounts
        .iter()
        .map(|account_meta| {
            let index_in_transaction = transaction_accounts
                .iter()
                .position(|(key, _account)| *key == account_meta.pubkey)
                .unwrap_or(transaction_accounts.len());
            InstructionAccount {
                index_in_transaction,
                index_in_caller: program_indices.len().saturating_add(index_in_transaction),
                is_signer: account_meta.is_signer,
                is_writable: account_meta.is_writable,
            }
        })
        .collect();
    MockInvokeContextPreparation {
        transaction_accounts,
        instruction_accounts,
    }
}

pub fn with_mock_invoke_context<R, F: FnMut(&mut InvokeContext) -> R>(
    loader_id: Pubkey,
    account_size: usize,
    mut callback: F,
) -> R {
    let program_indices = vec![0, 1];
    let transaction_accounts = vec![
        (
            loader_id,
            AccountSharedData::new(0, 0, &solana_sdk::native_loader::id()),
        ),
        (
            Pubkey::new_unique(),
            AccountSharedData::new(1, 0, &loader_id),
        ),
        (
            Pubkey::new_unique(),
            AccountSharedData::new(2, account_size, &Pubkey::new_unique()),
        ),
    ];
    let instruction_accounts = vec![AccountMeta {
        pubkey: transaction_accounts[2].0,
        is_signer: false,
        is_writable: false,
    }];
    let preparation =
        prepare_mock_invoke_context(transaction_accounts, instruction_accounts, &program_indices);
    let transaction_context = TransactionContext::new(
        preparation.transaction_accounts,
        ComputeBudget::default().max_invoke_depth,
    );
    let mut invoke_context = InvokeContext::new_mock(&transaction_context, &[]);
    invoke_context
        .push(&preparation.instruction_accounts, &program_indices)
        .unwrap();
    callback(&mut invoke_context)
}

pub fn mock_process_instruction_with_sysvars(
    loader_id: &Pubkey,
    mut program_indices: Vec<usize>,
    instruction_data: &[u8],
    transaction_accounts: Vec<TransactionAccount>,
    instruction_accounts: Vec<AccountMeta>,
    expected_result: Result<(), InstructionError>,
    sysvars: &[(Pubkey, Vec<u8>)],
    process_instruction: ProcessInstructionWithContext,
) -> Vec<AccountSharedData> {
    program_indices.insert(0, transaction_accounts.len());
    let mut preparation =
        prepare_mock_invoke_context(transaction_accounts, instruction_accounts, &program_indices);
    let processor_account = AccountSharedData::new(0, 0, &solana_sdk::native_loader::id());
    preparation
        .transaction_accounts
        .push((*loader_id, processor_account));
    let transaction_context = TransactionContext::new(
        preparation.transaction_accounts,
        ComputeBudget::default().max_invoke_depth,
    );
    let mut invoke_context = InvokeContext::new_mock(&transaction_context, &[]);
    invoke_context.sysvars = sysvars;
    let result = invoke_context
        .push(&preparation.instruction_accounts, &program_indices)
        .and_then(|_| process_instruction(1, instruction_data, &mut invoke_context));
    assert_eq!(result, expected_result);
    let mut transaction_accounts = transaction_context.deconstruct_without_keys().unwrap();
    transaction_accounts.pop();
    transaction_accounts
}

pub fn mock_process_instruction(
    loader_id: &Pubkey,
    program_indices: Vec<usize>,
    instruction_data: &[u8],
    transaction_accounts: Vec<TransactionAccount>,
    instruction_accounts: Vec<AccountMeta>,
    expected_result: Result<(), InstructionError>,
    process_instruction: ProcessInstructionWithContext,
) -> Vec<AccountSharedData> {
    mock_process_instruction_with_sysvars(
        loader_id,
        program_indices,
        instruction_data,
        transaction_accounts,
        instruction_accounts,
        expected_result,
        &[],
        process_instruction,
    )
}

/// Visit each unique instruction account index once
fn visit_each_account_once(
    instruction_accounts: &[InstructionAccount],
    work: &mut dyn FnMut(usize, &InstructionAccount) -> Result<(), InstructionError>,
) -> Result<(), InstructionError> {
    'root: for (index, instruction_account) in instruction_accounts.iter().enumerate() {
        // Note: This is an O(n^2) algorithm,
        // but performed on a very small slice and requires no heap allocations
        for before in instruction_accounts[..index].iter() {
            if before.index_in_transaction == instruction_account.index_in_transaction {
                continue 'root; // skip dups
            }
        }
        work(index, instruction_account)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        serde::{Deserialize, Serialize},
        solana_sdk::account::{ReadableAccount, WritableAccount},
    };

    #[derive(Debug, Serialize, Deserialize)]
    enum MockInstruction {
        NoopSuccess,
        NoopFail,
        ModifyOwned,
        ModifyNotOwned,
        ModifyReadonly,
        ConsumeComputeUnits {
            compute_units_consumed: u64,
            desired_result: Result<(), InstructionError>,
        },
    }

    #[test]
    fn test_visit_each_account_once() {
        let do_work = |accounts: &[InstructionAccount]| -> (usize, usize, usize) {
            let mut unique_entries = 0;
            let mut index_sum_a = 0;
            let mut index_sum_b = 0;
            let mut work = |index_in_instruction: usize, entry: &InstructionAccount| {
                unique_entries += 1;
                index_sum_a += index_in_instruction;
                index_sum_b += entry.index_in_transaction;
                Ok(())
            };
            visit_each_account_once(accounts, &mut work).unwrap();

            (unique_entries, index_sum_a, index_sum_b)
        };

        assert_eq!(
            (3, 3, 19),
            do_work(&[
                InstructionAccount {
                    index_in_transaction: 7,
                    index_in_caller: 0,
                    is_signer: false,
                    is_writable: false,
                },
                InstructionAccount {
                    index_in_transaction: 3,
                    index_in_caller: 1,
                    is_signer: false,
                    is_writable: false,
                },
                InstructionAccount {
                    index_in_transaction: 9,
                    index_in_caller: 2,
                    is_signer: false,
                    is_writable: false,
                },
                InstructionAccount {
                    index_in_transaction: 3,
                    index_in_caller: 1,
                    is_signer: false,
                    is_writable: false,
                },
            ])
        );
    }

    #[test]
    fn test_program_entry_debug() {
        #[allow(clippy::unnecessary_wraps)]
        fn mock_process_instruction(
            _first_instruction_account: usize,
            _data: &[u8],
            _invoke_context: &mut InvokeContext,
        ) -> Result<(), InstructionError> {
            Ok(())
        }
        #[allow(clippy::unnecessary_wraps)]
        fn mock_ix_processor(
            _first_instruction_account: usize,
            _data: &[u8],
            _context: &mut InvokeContext,
        ) -> Result<(), InstructionError> {
            Ok(())
        }
        let builtin_programs = &[
            BuiltinProgram {
                program_id: solana_sdk::pubkey::new_rand(),
                process_instruction: mock_process_instruction,
            },
            BuiltinProgram {
                program_id: solana_sdk::pubkey::new_rand(),
                process_instruction: mock_ix_processor,
            },
        ];
        assert!(!format!("{:?}", builtin_programs).is_empty());
    }

    #[allow(clippy::integer_arithmetic)]
    fn mock_process_instruction(
        first_instruction_account: usize,
        data: &[u8],
        invoke_context: &mut InvokeContext,
    ) -> Result<(), InstructionError> {
        let program_id = invoke_context.get_caller()?;
        let keyed_accounts = invoke_context.get_keyed_accounts()?;
        assert_eq!(
            *program_id,
            keyed_account_at_index(keyed_accounts, first_instruction_account)?.owner()?
        );
        assert_ne!(
            keyed_account_at_index(keyed_accounts, first_instruction_account + 1)?.owner()?,
            *keyed_account_at_index(keyed_accounts, first_instruction_account)?.unsigned_key()
        );

        if let Ok(instruction) = bincode::deserialize(data) {
            match instruction {
                MockInstruction::NoopSuccess => (),
                MockInstruction::NoopFail => return Err(InstructionError::GenericError),
                MockInstruction::ModifyOwned => {
                    keyed_account_at_index(keyed_accounts, first_instruction_account)?
                        .try_account_ref_mut()?
                        .data_as_mut_slice()[0] = 1
                }
                MockInstruction::ModifyNotOwned => {
                    keyed_account_at_index(keyed_accounts, first_instruction_account + 1)?
                        .try_account_ref_mut()?
                        .data_as_mut_slice()[0] = 1
                }
                MockInstruction::ModifyReadonly => {
                    keyed_account_at_index(keyed_accounts, first_instruction_account + 2)?
                        .try_account_ref_mut()?
                        .data_as_mut_slice()[0] = 1
                }
                MockInstruction::ConsumeComputeUnits {
                    compute_units_consumed,
                    desired_result,
                } => {
                    invoke_context
                        .get_compute_meter()
                        .borrow_mut()
                        .consume(compute_units_consumed)
                        .unwrap();
                    return desired_result;
                }
            }
        } else {
            return Err(InstructionError::InvalidInstructionData);
        }
        Ok(())
    }

    #[test]
    fn test_invoke_context() {
        const MAX_DEPTH: usize = 10;
        let mut invoke_stack = vec![];
        let mut accounts = vec![];
        let mut instruction_accounts = vec![];
        for index in 0..MAX_DEPTH {
            invoke_stack.push(solana_sdk::pubkey::new_rand());
            accounts.push((
                solana_sdk::pubkey::new_rand(),
                AccountSharedData::new(index as u64, 1, &invoke_stack[index]),
            ));
            instruction_accounts.push(InstructionAccount {
                index_in_transaction: index,
                index_in_caller: 1 + index,
                is_signer: false,
                is_writable: true,
            });
        }
        for (index, program_id) in invoke_stack.iter().enumerate() {
            accounts.push((
                *program_id,
                AccountSharedData::new(1, 1, &solana_sdk::pubkey::Pubkey::default()),
            ));
            instruction_accounts.push(InstructionAccount {
                index_in_transaction: index,
                index_in_caller: 1 + index,
                is_signer: false,
                is_writable: false,
            });
        }
        let transaction_context = TransactionContext::new(accounts, MAX_DEPTH);
        let mut invoke_context = InvokeContext::new_mock(&transaction_context, &[]);

        // Check call depth increases and has a limit
        let mut depth_reached = 0;
        for _ in 0..invoke_stack.len() {
            if Err(InstructionError::CallDepth)
                == invoke_context.push(&instruction_accounts, &[MAX_DEPTH + depth_reached])
            {
                break;
            }
            depth_reached += 1;
        }
        assert_ne!(depth_reached, 0);
        assert!(depth_reached < MAX_DEPTH);

        // Mock each invocation
        for owned_index in (1..depth_reached).rev() {
            let not_owned_index = owned_index - 1;
            let instruction_accounts = vec![
                InstructionAccount {
                    index_in_transaction: not_owned_index,
                    index_in_caller: 1 + not_owned_index,
                    is_signer: false,
                    is_writable: true,
                },
                InstructionAccount {
                    index_in_transaction: owned_index,
                    index_in_caller: 1 + owned_index,
                    is_signer: false,
                    is_writable: true,
                },
            ];

            // modify account owned by the program
            transaction_context
                .get_account_at_index(owned_index)
                .borrow_mut()
                .data_as_mut_slice()[0] = (MAX_DEPTH + owned_index) as u8;
            invoke_context
                .verify_and_update(&instruction_accounts, None)
                .unwrap();
            assert_eq!(
                invoke_context.pre_accounts[owned_index].data()[0],
                (MAX_DEPTH + owned_index) as u8
            );

            // modify account not owned by the program
            let data = transaction_context
                .get_account_at_index(not_owned_index)
                .borrow_mut()
                .data()[0];
            transaction_context
                .get_account_at_index(not_owned_index)
                .borrow_mut()
                .data_as_mut_slice()[0] = (MAX_DEPTH + not_owned_index) as u8;
            assert_eq!(
                invoke_context.verify_and_update(&instruction_accounts, None),
                Err(InstructionError::ExternalAccountDataModified)
            );
            assert_eq!(invoke_context.pre_accounts[not_owned_index].data()[0], data);
            transaction_context
                .get_account_at_index(not_owned_index)
                .borrow_mut()
                .data_as_mut_slice()[0] = data;

            invoke_context.pop();
        }
    }

    #[test]
    fn test_invoke_context_verify() {
        let accounts = vec![(solana_sdk::pubkey::new_rand(), AccountSharedData::default())];
        let instruction_accounts = vec![];
        let program_indices = vec![0];
        let transaction_context = TransactionContext::new(accounts, 1);
        let mut invoke_context = InvokeContext::new_mock(&transaction_context, &[]);
        invoke_context
            .push(&instruction_accounts, &program_indices)
            .unwrap();
        assert!(invoke_context
            .verify(&instruction_accounts, &program_indices)
            .is_ok());

        let mut _borrowed = transaction_context.get_account_at_index(0).borrow();
        assert_eq!(
            invoke_context.verify(&instruction_accounts, &program_indices),
            Err(InstructionError::AccountBorrowOutstanding)
        );
    }

    #[test]
    fn test_process_cross_program() {
        let caller_program_id = solana_sdk::pubkey::new_rand();
        let callee_program_id = solana_sdk::pubkey::new_rand();
        let builtin_programs = &[BuiltinProgram {
            program_id: callee_program_id,
            process_instruction: mock_process_instruction,
        }];

        let owned_account = AccountSharedData::new(42, 1, &callee_program_id);
        let not_owned_account = AccountSharedData::new(84, 1, &solana_sdk::pubkey::new_rand());
        let readonly_account = AccountSharedData::new(168, 1, &solana_sdk::pubkey::new_rand());
        let loader_account = AccountSharedData::new(0, 0, &native_loader::id());
        let mut program_account = AccountSharedData::new(1, 0, &native_loader::id());
        program_account.set_executable(true);

        let accounts = vec![
            (solana_sdk::pubkey::new_rand(), owned_account),
            (solana_sdk::pubkey::new_rand(), not_owned_account),
            (solana_sdk::pubkey::new_rand(), readonly_account),
            (caller_program_id, loader_account),
            (callee_program_id, program_account),
        ];
        let program_indices = [3, 4];

        let metas = vec![
            AccountMeta::new(accounts[0].0, false),
            AccountMeta::new(accounts[1].0, false),
            AccountMeta::new_readonly(accounts[2].0, false),
        ];
        let instruction_accounts = metas
            .iter()
            .enumerate()
            .map(|(index_in_transaction, account_meta)| InstructionAccount {
                index_in_transaction,
                index_in_caller: program_indices.len() + index_in_transaction,
                is_signer: account_meta.is_signer,
                is_writable: account_meta.is_writable,
            })
            .collect::<Vec<_>>();
        let instruction = Instruction::new_with_bincode(
            callee_program_id,
            &MockInstruction::NoopSuccess,
            metas.clone(),
        );
        let transaction_context = TransactionContext::new(accounts, 1);
        let mut invoke_context = InvokeContext::new_mock(&transaction_context, builtin_programs);
        invoke_context
            .push(&instruction_accounts, &program_indices[..1])
            .unwrap();

        // not owned account modified by the caller (before the invoke)
        transaction_context
            .get_account_at_index(1)
            .borrow_mut()
            .data_as_mut_slice()[0] = 1;
        assert_eq!(
            invoke_context
                .process_instruction(
                    &instruction.data,
                    &instruction_accounts,
                    None,
                    &program_indices[1..],
                )
                .result,
            Err(InstructionError::ExternalAccountDataModified)
        );
        transaction_context
            .get_account_at_index(1)
            .borrow_mut()
            .data_as_mut_slice()[0] = 0;

        // readonly account modified by the invoker
        transaction_context
            .get_account_at_index(2)
            .borrow_mut()
            .data_as_mut_slice()[0] = 1;
        assert_eq!(
            invoke_context
                .process_instruction(
                    &instruction.data,
                    &instruction_accounts,
                    None,
                    &program_indices[1..],
                )
                .result,
            Err(InstructionError::ReadonlyDataModified)
        );
        transaction_context
            .get_account_at_index(2)
            .borrow_mut()
            .data_as_mut_slice()[0] = 0;

        invoke_context.pop();

        let cases = vec![
            (
                MockInstruction::NoopSuccess,
                ProcessInstructionResult {
                    result: Ok(()),
                    compute_units_consumed: 0,
                },
            ),
            (
                MockInstruction::NoopFail,
                ProcessInstructionResult {
                    result: Err(InstructionError::GenericError),
                    compute_units_consumed: 0,
                },
            ),
            (
                MockInstruction::ModifyOwned,
                ProcessInstructionResult {
                    result: Ok(()),
                    compute_units_consumed: 0,
                },
            ),
            (
                MockInstruction::ModifyNotOwned,
                ProcessInstructionResult {
                    result: Err(InstructionError::ExternalAccountDataModified),
                    compute_units_consumed: 0,
                },
            ),
        ];
        for case in cases {
            let instruction =
                Instruction::new_with_bincode(callee_program_id, &case.0, metas.clone());
            invoke_context
                .push(&instruction_accounts, &program_indices[..1])
                .unwrap();
            assert_eq!(
                invoke_context.process_instruction(
                    &instruction.data,
                    &instruction_accounts,
                    None,
                    &program_indices[1..],
                ),
                case.1
            );
            invoke_context.pop();
        }
    }

    #[test]
    fn test_native_invoke() {
        let callee_program_id = solana_sdk::pubkey::new_rand();
        let builtin_programs = &[BuiltinProgram {
            program_id: callee_program_id,
            process_instruction: mock_process_instruction,
        }];

        let owned_account = AccountSharedData::new(42, 1, &callee_program_id);
        let not_owned_account = AccountSharedData::new(84, 1, &solana_sdk::pubkey::new_rand());
        let readonly_account = AccountSharedData::new(168, 1, &solana_sdk::pubkey::new_rand());
        let mut program_account = AccountSharedData::new(1, 0, &native_loader::id());
        program_account.set_executable(true);
        let accounts = vec![
            (solana_sdk::pubkey::new_rand(), owned_account),
            (solana_sdk::pubkey::new_rand(), not_owned_account),
            (solana_sdk::pubkey::new_rand(), readonly_account),
            (callee_program_id, program_account),
        ];
        let program_indices = [3];

        let metas = vec![
            AccountMeta::new(accounts[0].0, false),
            AccountMeta::new(accounts[1].0, false),
            AccountMeta::new_readonly(accounts[2].0, false),
            AccountMeta::new_readonly(accounts[3].0, false),
        ];
        let instruction_accounts = metas
            .iter()
            .enumerate()
            .map(|(index_in_transaction, account_meta)| InstructionAccount {
                index_in_transaction,
                index_in_caller: program_indices.len() + index_in_transaction,
                is_signer: account_meta.is_signer,
                is_writable: account_meta.is_writable,
            })
            .collect::<Vec<_>>();
        let callee_instruction = Instruction::new_with_bincode(
            callee_program_id,
            &MockInstruction::NoopSuccess,
            metas.clone(),
        );

        let transaction_context = TransactionContext::new(accounts, 1);
        let mut invoke_context = InvokeContext::new_mock(&transaction_context, builtin_programs);
        invoke_context
            .push(&instruction_accounts, &program_indices)
            .unwrap();

        // not owned account modified by the invoker
        transaction_context
            .get_account_at_index(1)
            .borrow_mut()
            .data_as_mut_slice()[0] = 1;
        assert_eq!(
            invoke_context.native_invoke(callee_instruction.clone(), &[]),
            Err(InstructionError::ExternalAccountDataModified)
        );
        transaction_context
            .get_account_at_index(1)
            .borrow_mut()
            .data_as_mut_slice()[0] = 0;

        // readonly account modified by the invoker
        transaction_context
            .get_account_at_index(2)
            .borrow_mut()
            .data_as_mut_slice()[0] = 1;
        assert_eq!(
            invoke_context.native_invoke(callee_instruction, &[]),
            Err(InstructionError::ReadonlyDataModified)
        );
        transaction_context
            .get_account_at_index(2)
            .borrow_mut()
            .data_as_mut_slice()[0] = 0;

        invoke_context.pop();

        // Other test cases
        let cases = vec![
            (MockInstruction::NoopSuccess, Ok(())),
            (
                MockInstruction::NoopFail,
                Err(InstructionError::GenericError),
            ),
            (MockInstruction::ModifyOwned, Ok(())),
            (
                MockInstruction::ModifyNotOwned,
                Err(InstructionError::ExternalAccountDataModified),
            ),
            (
                MockInstruction::ModifyReadonly,
                Err(InstructionError::ReadonlyDataModified),
            ),
        ];
        for case in cases {
            let callee_instruction =
                Instruction::new_with_bincode(callee_program_id, &case.0, metas.clone());
            invoke_context
                .push(&instruction_accounts, &program_indices)
                .unwrap();
            assert_eq!(
                invoke_context.native_invoke(callee_instruction, &[]),
                case.1
            );
            invoke_context.pop();
        }
    }

    #[test]
    fn test_invoke_context_compute_budget() {
        let accounts = vec![
            (solana_sdk::pubkey::new_rand(), AccountSharedData::default()),
            (crate::neon_evm_program::id(), AccountSharedData::default()),
        ];

        let mut feature_set = FeatureSet::all_enabled();
        feature_set.deactivate(&tx_wide_compute_cap::id());
        feature_set.deactivate(&requestable_heap_size::id());
        let transaction_context = TransactionContext::new(accounts, 1);
        let mut invoke_context = InvokeContext::new_mock(&transaction_context, &[]);
        invoke_context.feature_set = Arc::new(feature_set);

        invoke_context.push(&[], &[0]).unwrap();
        assert_eq!(
            *invoke_context.get_compute_budget(),
            ComputeBudget::default()
        );
        invoke_context.pop();

        invoke_context.push(&[], &[1]).unwrap();
        let expected_compute_budget = ComputeBudget {
            max_units: 500_000,
            heap_size: Some(256_usize.saturating_mul(1024)),
            ..ComputeBudget::default()
        };
        assert_eq!(
            *invoke_context.get_compute_budget(),
            expected_compute_budget
        );
        invoke_context.pop();

        invoke_context.push(&[], &[0]).unwrap();
        assert_eq!(
            *invoke_context.get_compute_budget(),
            ComputeBudget::default()
        );
        invoke_context.pop();
    }

    #[test]
    fn test_process_instruction_compute_budget() {
        let caller_program_id = solana_sdk::pubkey::new_rand();
        let callee_program_id = solana_sdk::pubkey::new_rand();
        let builtin_programs = &[BuiltinProgram {
            program_id: callee_program_id,
            process_instruction: mock_process_instruction,
        }];

        let owned_account = AccountSharedData::new(42, 1, &callee_program_id);
        let not_owned_account = AccountSharedData::new(84, 1, &solana_sdk::pubkey::new_rand());
        let readonly_account = AccountSharedData::new(168, 1, &solana_sdk::pubkey::new_rand());
        let loader_account = AccountSharedData::new(0, 0, &native_loader::id());
        let mut program_account = AccountSharedData::new(1, 0, &native_loader::id());
        program_account.set_executable(true);

        let accounts = vec![
            (solana_sdk::pubkey::new_rand(), owned_account),
            (solana_sdk::pubkey::new_rand(), not_owned_account),
            (solana_sdk::pubkey::new_rand(), readonly_account),
            (caller_program_id, loader_account),
            (callee_program_id, program_account),
        ];
        let program_indices = [3, 4];

        let metas = vec![
            AccountMeta::new(accounts[0].0, false),
            AccountMeta::new(accounts[1].0, false),
            AccountMeta::new_readonly(accounts[2].0, false),
        ];
        let instruction_accounts = metas
            .iter()
            .enumerate()
            .map(|(account_index, account_meta)| InstructionAccount {
                index: account_index,
                is_signer: account_meta.is_signer,
                is_writable: account_meta.is_writable,
            })
            .collect::<Vec<_>>();

        let transaction_context = TransactionContext::new(accounts, 1);
        let mut invoke_context = InvokeContext::new_mock(&transaction_context, builtin_programs);
        let compute_units_consumed = 10;
        let desired_results = vec![Ok(()), Err(InstructionError::GenericError)];

        for desired_result in desired_results {
            let instruction = Instruction::new_with_bincode(
                callee_program_id,
                &MockInstruction::ConsumeComputeUnits {
                    compute_units_consumed,
                    desired_result: desired_result.clone(),
                },
                metas.clone(),
            );
            invoke_context
                .push(&instruction_accounts, &program_indices[..1])
                .unwrap();

            let result = invoke_context.process_instruction(
                &instruction.data,
                &instruction_accounts,
                None,
                &program_indices[1..],
            );

            // Because the instruction had compute cost > 0, then regardless of the execution result,
            // the number of compute units consumed should be a non-default which is something greater
            // than zero.
            assert!(result.compute_units_consumed > 0);
            assert_eq!(
                result,
                ProcessInstructionResult {
                    compute_units_consumed,
                    result: desired_result,
                }
            );
        }
    }
}
