#![deny(clippy::integer_arithmetic)]
#![deny(clippy::indexing_slicing)]

pub mod allocator_bump;
pub mod deprecated;
pub mod serialization;
pub mod syscalls;
pub mod upgradeable;
pub mod upgradeable_with_jit;
pub mod with_jit;

use {
    crate::{
        allocator_bump::BpfAllocator,
        serialization::{deserialize_parameters, serialize_parameters},
        syscalls::SyscallError,
    },
    solana_measure::measure::Measure,
    solana_program_runtime::{
        compute_budget::ComputeBudget,
        executor_cache::TransactionExecutorCache,
        ic_logger_msg, ic_msg,
        invoke_context::InvokeContext,
        loaded_programs::{LoadProgramMetrics, LoadedProgram, LoadedProgramType},
        log_collector::LogCollector,
        stable_log,
        sysvar_cache::get_sysvar_with_account_check,
    },
    solana_rbpf::{
        aligned_memory::AlignedMemory,
        ebpf::{self, HOST_ALIGN, MM_HEAP_START},
        elf::Executable,
        error::{EbpfError, UserDefinedError},
        memory_region::{MemoryCowCallback, MemoryMapping, MemoryRegion},
        verifier::{RequisiteVerifier, VerifierError},
        vm::{ContextObject, EbpfVm, ProgramResult, VerifiedExecutable},
    },
    solana_sdk::{
        bpf_loader, bpf_loader_deprecated,
        bpf_loader_upgradeable::{self, UpgradeableLoaderState},
        clock::Slot,
        entrypoint::SUCCESS,
        feature_set::{
            cap_accounts_data_allocations_per_transaction, cap_bpf_program_instruction_accounts,
            check_slice_translation_size, delay_visibility_of_program_deployment,
            disable_deploy_of_alloc_free_syscall, enable_bpf_loader_extend_program_ix,
            enable_bpf_loader_set_authority_checked_ix, enable_program_redeployment_cooldown,
            limit_max_instruction_trace_length, native_programs_consume_cu,
            remove_bpf_loader_incorrect_program_id, round_up_heap_size, FeatureSet,
        },
        instruction::{AccountMeta, InstructionError},
        loader_instruction::LoaderInstruction,
        loader_upgradeable_instruction::UpgradeableLoaderInstruction,
        native_loader,
        program_error::{
            MAX_ACCOUNTS_DATA_ALLOCATIONS_EXCEEDED, MAX_INSTRUCTION_TRACE_LENGTH_EXCEEDED,
        },
        program_utils::limited_deserialize,
        pubkey::Pubkey,
        saturating_add_assign,
        system_instruction::{self, MAX_PERMITTED_DATA_LENGTH},
        transaction_context::{
            BorrowedAccount, IndexOfAccount, InstructionContext, TransactionContext,
        },
    },
    std::{
        cell::{RefCell, RefMut},
        fmt::Debug,
        mem,
        rc::Rc,
        sync::{atomic::Ordering, Arc},
    },
    thiserror::Error,
};

solana_sdk::declare_builtin!(
    solana_sdk::bpf_loader::ID,
    solana_bpf_loader_program,
    solana_bpf_loader_program::process_instruction
);

/// Errors returned by functions the BPF Loader registers with the VM
#[derive(Debug, Error, PartialEq, Eq)]
pub enum BpfError {
    #[error("{0}")]
    VerifierError(#[from] VerifierError),
    #[error("{0}")]
    SyscallError(#[from] SyscallError),
}
impl UserDefinedError for BpfError {}

#[allow(clippy::too_many_arguments)]
pub fn load_program_from_bytes(
    feature_set: &FeatureSet,
    compute_budget: &ComputeBudget,
    log_collector: Option<Rc<RefCell<LogCollector>>>,
    load_program_metrics: &mut LoadProgramMetrics,
    programdata: &[u8],
    loader_key: &Pubkey,
    account_size: usize,
    deployment_slot: Slot,
    use_jit: bool,
    reject_deployment_of_broken_elfs: bool,
) -> Result<LoadedProgram, InstructionError> {
    let mut register_syscalls_time = Measure::start("register_syscalls_time");
    let disable_deploy_of_alloc_free_syscall = reject_deployment_of_broken_elfs
        && feature_set.is_active(&disable_deploy_of_alloc_free_syscall::id());
    let loader = syscalls::create_loader(
        feature_set,
        compute_budget,
        reject_deployment_of_broken_elfs,
        disable_deploy_of_alloc_free_syscall,
        false,
    )
    .map_err(|e| {
        ic_logger_msg!(log_collector, "Failed to register syscalls: {}", e);
        InstructionError::ProgramEnvironmentSetupFailure
    })?;
    register_syscalls_time.stop();
    load_program_metrics.register_syscalls_us = register_syscalls_time.as_us();

    let loaded_program = LoadedProgram::new(
        loader_key,
        loader,
        deployment_slot,
        programdata,
        account_size,
        use_jit,
        load_program_metrics,
    )
    .map_err(|err| {
        ic_logger_msg!(log_collector, "{}", err);
        InstructionError::InvalidAccountData
    })?;
    Ok(loaded_program)
}

fn get_programdata_offset_and_depoyment_offset(
    log_collector: &Option<Rc<RefCell<LogCollector>>>,
    program: &BorrowedAccount,
    programdata: &BorrowedAccount,
) -> Result<(usize, Slot), InstructionError> {
    if bpf_loader_upgradeable::check_id(program.get_owner()) {
        if let UpgradeableLoaderState::Program {
            programdata_address: _,
        } = program.get_state()?
        {
            if let UpgradeableLoaderState::ProgramData {
                slot,
                upgrade_authority_address: _,
            } = programdata.get_state()?
            {
                Ok((UpgradeableLoaderState::size_of_programdata_metadata(), slot))
            } else {
                ic_logger_msg!(log_collector, "Program has been closed");
                Err(InstructionError::InvalidAccountData)
            }
        } else {
            ic_logger_msg!(log_collector, "Invalid Program account");
            Err(InstructionError::InvalidAccountData)
        }
    } else {
        Ok((0, 0))
    }
}

pub fn load_program_from_account(
    feature_set: &FeatureSet,
    compute_budget: &ComputeBudget,
    log_collector: Option<Rc<RefCell<LogCollector>>>,
    tx_executor_cache: Option<RefMut<TransactionExecutorCache>>,
    program: &BorrowedAccount,
    programdata: &BorrowedAccount,
    use_jit: bool,
) -> Result<(Arc<LoadedProgram>, Option<LoadProgramMetrics>), InstructionError> {
    if !check_loader_id(program.get_owner()) {
        ic_logger_msg!(
            log_collector,
            "Executable account not owned by the BPF loader"
        );
        return Err(InstructionError::IncorrectProgramId);
    }

    let (programdata_offset, deployment_slot) =
        get_programdata_offset_and_depoyment_offset(&log_collector, program, programdata)?;

    if let Some(ref tx_executor_cache) = tx_executor_cache {
        if let Some(loaded_program) = tx_executor_cache.get(program.get_key()) {
            if loaded_program.is_tombstone() {
                // We cached that the Executor does not exist, abort
                // This case can only happen once delay_visibility_of_program_deployment is active.
                return Err(InstructionError::InvalidAccountData);
            }
            // Executor exists and is cached, use it
            return Ok((loaded_program, None));
        }
    }

    let programdata_size = if programdata_offset != 0 {
        programdata.get_data().len()
    } else {
        0
    };

    let mut load_program_metrics = LoadProgramMetrics {
        program_id: program.get_key().to_string(),
        ..LoadProgramMetrics::default()
    };

    let loaded_program = Arc::new(load_program_from_bytes(
        feature_set,
        compute_budget,
        log_collector,
        &mut load_program_metrics,
        programdata
            .get_data()
            .get(programdata_offset..)
            .ok_or(InstructionError::AccountDataTooSmall)?,
        program.get_owner(),
        program.get_data().len().saturating_add(programdata_size),
        deployment_slot,
        use_jit,
        false, /* reject_deployment_of_broken_elfs */
    )?);
    if let Some(mut tx_executor_cache) = tx_executor_cache {
        tx_executor_cache.set(
            *program.get_key(),
            loaded_program.clone(),
            false,
            feature_set.is_active(&delay_visibility_of_program_deployment::id()),
            deployment_slot,
        );
    }

    Ok((loaded_program, Some(load_program_metrics)))
}

macro_rules! deploy_program {
    ($invoke_context:expr, $use_jit:expr, $program_id:expr, $loader_key:expr,
     $account_size:expr, $slot:expr, $drop:expr, $new_programdata:expr $(,)?) => {{
        let delay_visibility_of_program_deployment = $invoke_context
            .feature_set
            .is_active(&delay_visibility_of_program_deployment::id());
        let mut load_program_metrics = LoadProgramMetrics::default();
        let executor = load_program_from_bytes(
            &$invoke_context.feature_set,
            $invoke_context.get_compute_budget(),
            $invoke_context.get_log_collector(),
            &mut load_program_metrics,
            $new_programdata,
            $loader_key,
            $account_size,
            $slot,
            $use_jit,
            true,
        )?;
        $drop
        load_program_metrics.program_id = $program_id.to_string();
        load_program_metrics.submit_datapoint(&mut $invoke_context.timings);
        $invoke_context.tx_executor_cache.borrow_mut().set(
            $program_id,
            Arc::new(executor),
            true,
            delay_visibility_of_program_deployment,
            $slot,
        );
    }};
}

fn write_program_data(
    program_data_offset: usize,
    bytes: &[u8],
    invoke_context: &mut InvokeContext,
) -> Result<(), InstructionError> {
    let transaction_context = &invoke_context.transaction_context;
    let instruction_context = transaction_context.get_current_instruction_context()?;
    let mut program = instruction_context.try_borrow_instruction_account(transaction_context, 0)?;
    let data = program.get_data_mut()?;
    let write_offset = program_data_offset.saturating_add(bytes.len());
    if data.len() < write_offset {
        ic_msg!(
            invoke_context,
            "Write overflow: {} < {}",
            data.len(),
            write_offset,
        );
        return Err(InstructionError::AccountDataTooSmall);
    }
    data.get_mut(program_data_offset..write_offset)
        .ok_or(InstructionError::AccountDataTooSmall)?
        .copy_from_slice(bytes);
    Ok(())
}

fn check_loader_id(id: &Pubkey) -> bool {
    bpf_loader::check_id(id)
        || bpf_loader_deprecated::check_id(id)
        || bpf_loader_upgradeable::check_id(id)
}

fn calculate_heap_cost(heap_size: u64, heap_cost: u64, enable_rounding_fix: bool) -> u64 {
    const KIBIBYTE: u64 = 1024;
    const PAGE_SIZE_KB: u64 = 32;
    let mut rounded_heap_size = heap_size;
    if enable_rounding_fix {
        rounded_heap_size = rounded_heap_size
            .saturating_add(PAGE_SIZE_KB.saturating_mul(KIBIBYTE).saturating_sub(1));
    }
    rounded_heap_size
        .saturating_div(PAGE_SIZE_KB.saturating_mul(KIBIBYTE))
        .saturating_sub(1)
        .saturating_mul(heap_cost)
}

/// Create the SBF virtual machine
pub fn create_ebpf_vm<'a, 'b>(
    program: &'a VerifiedExecutable<RequisiteVerifier, InvokeContext<'b>>,
    stack: &'a mut AlignedMemory<HOST_ALIGN>,
    heap: AlignedMemory<HOST_ALIGN>,
    regions: Vec<MemoryRegion>,
    orig_account_lengths: Vec<usize>,
    invoke_context: &'a mut InvokeContext<'b>,
) -> Result<EbpfVm<'a, RequisiteVerifier, InvokeContext<'b>>, EbpfError> {
    let round_up_heap_size = invoke_context
        .feature_set
        .is_active(&round_up_heap_size::id());
    let heap_cost_result = invoke_context.consume_checked(calculate_heap_cost(
        heap.len() as u64,
        invoke_context.get_compute_budget().heap_cost,
        round_up_heap_size,
    ));
    if round_up_heap_size {
        heap_cost_result.map_err(SyscallError::InstructionError)?;
    }
    let check_aligned = bpf_loader_deprecated::id()
        != invoke_context
            .transaction_context
            .get_current_instruction_context()
            .and_then(|instruction_context| {
                instruction_context
                    .try_borrow_last_program_account(invoke_context.transaction_context)
            })
            .map(|program_account| *program_account.get_owner())
            .map_err(SyscallError::InstructionError)?;
    let check_size = invoke_context
        .feature_set
        .is_active(&check_slice_translation_size::id());
    let allocator = Rc::new(RefCell::new(BpfAllocator::new(heap, MM_HEAP_START)));
    invoke_context
        .set_syscall_context(
            check_aligned,
            check_size,
            orig_account_lengths,
            allocator.clone(),
        )
        .map_err(SyscallError::InstructionError)?;
    let stack_len = stack.len();
    let memory_mapping = create_memory_mapping(
        program.get_executable(),
        stack,
        allocator.borrow_mut().heap_mut(),
        regions,
        None,
    )?;

    EbpfVm::new(program, invoke_context, memory_mapping, stack_len)
}

#[macro_export]
macro_rules! create_vm {
    ($vm_name:ident, $executable:expr, $stack:ident, $heap:ident, $additional_regions:expr, $orig_account_lengths:expr, $invoke_context:expr) => {
        let mut $stack = solana_rbpf::aligned_memory::AlignedMemory::<
            { solana_rbpf::ebpf::HOST_ALIGN },
        >::zero_filled($executable.get_executable().get_config().stack_size());

        // this is needed if the caller passes "&mut invoke_context" to the
        // macro. The lint complains that (&mut invoke_context).get_compute_budget()
        // does an unnecessary mutable borrow
        #[allow(clippy::unnecessary_mut_passed)]
        let heap_size = $invoke_context
            .get_compute_budget()
            .heap_size
            .unwrap_or(solana_sdk::entrypoint::HEAP_LENGTH);
        let $heap = solana_rbpf::aligned_memory::AlignedMemory::<{ solana_rbpf::ebpf::HOST_ALIGN }>::zero_filled(heap_size);

        let $vm_name = create_ebpf_vm(
            $executable,
            &mut $stack,
            $heap,
            $additional_regions,
            $orig_account_lengths,
            $invoke_context,
        );
    };
}

fn create_memory_mapping<'a, 'b, C: ContextObject>(
    executable: &'a Executable<C>,
    stack: &'b mut AlignedMemory<{ HOST_ALIGN }>,
    heap: &'b mut AlignedMemory<{ HOST_ALIGN }>,
    additional_regions: Vec<MemoryRegion>,
    cow_cb: Option<MemoryCowCallback>,
) -> Result<MemoryMapping<'a>, EbpfError> {
    let config = executable.get_config();
    let regions: Vec<MemoryRegion> = vec![
        executable.get_ro_region(),
        MemoryRegion::new_writable_gapped(
            stack.as_slice_mut(),
            ebpf::MM_STACK_START,
            if !config.dynamic_stack_frames && config.enable_stack_frame_gaps {
                config.stack_frame_size as u64
            } else {
                0
            },
        ),
        MemoryRegion::new_writable(heap.as_slice_mut(), ebpf::MM_HEAP_START),
    ]
    .into_iter()
    .chain(additional_regions.into_iter())
    .collect();

    Ok(if let Some(cow_cb) = cow_cb {
        MemoryMapping::new_with_cow(regions, cow_cb, config)?
    } else {
        MemoryMapping::new(regions, config)?
    })
}

pub fn process_instruction(invoke_context: &mut InvokeContext) -> Result<(), InstructionError> {
    process_instruction_common(invoke_context, false)
}

pub fn process_instruction_jit(invoke_context: &mut InvokeContext) -> Result<(), InstructionError> {
    process_instruction_common(invoke_context, true)
}

fn process_instruction_common(
    invoke_context: &mut InvokeContext,
    use_jit: bool,
) -> Result<(), InstructionError> {
    let log_collector = invoke_context.get_log_collector();
    let transaction_context = &invoke_context.transaction_context;
    let instruction_context = transaction_context.get_current_instruction_context()?;

    if !invoke_context
        .feature_set
        .is_active(&remove_bpf_loader_incorrect_program_id::id())
    {
        fn get_index_in_transaction(
            instruction_context: &InstructionContext,
            index_in_instruction: IndexOfAccount,
        ) -> Result<IndexOfAccount, InstructionError> {
            if index_in_instruction < instruction_context.get_number_of_program_accounts() {
                instruction_context
                    .get_index_of_program_account_in_transaction(index_in_instruction)
            } else {
                instruction_context.get_index_of_instruction_account_in_transaction(
                    index_in_instruction
                        .saturating_sub(instruction_context.get_number_of_program_accounts()),
                )
            }
        }

        fn try_borrow_account<'a>(
            transaction_context: &'a TransactionContext,
            instruction_context: &'a InstructionContext,
            index_in_instruction: IndexOfAccount,
        ) -> Result<BorrowedAccount<'a>, InstructionError> {
            if index_in_instruction < instruction_context.get_number_of_program_accounts() {
                instruction_context
                    .try_borrow_program_account(transaction_context, index_in_instruction)
            } else {
                instruction_context.try_borrow_instruction_account(
                    transaction_context,
                    index_in_instruction
                        .saturating_sub(instruction_context.get_number_of_program_accounts()),
                )
            }
        }

        let first_instruction_account = {
            let borrowed_root_account =
                instruction_context.try_borrow_program_account(transaction_context, 0)?;
            let owner_id = borrowed_root_account.get_owner();
            if native_loader::check_id(owner_id) {
                1
            } else {
                0
            }
        };
        let first_account_key = transaction_context.get_key_of_account_at_index(
            get_index_in_transaction(instruction_context, first_instruction_account)?,
        )?;
        let second_account_key = get_index_in_transaction(
            instruction_context,
            first_instruction_account.saturating_add(1),
        )
        .and_then(|index_in_transaction| {
            transaction_context.get_key_of_account_at_index(index_in_transaction)
        });
        let program_id = instruction_context.get_last_program_key(transaction_context)?;
        if first_account_key == program_id
            || second_account_key
                .map(|key| key == program_id)
                .unwrap_or(false)
        {
        } else {
            let first_account = try_borrow_account(
                transaction_context,
                instruction_context,
                first_instruction_account,
            )?;
            if first_account.is_executable() {
                ic_logger_msg!(log_collector, "BPF loader is executable");
                return Err(InstructionError::IncorrectProgramId);
            }
        }
    }

    let program_account =
        instruction_context.try_borrow_last_program_account(transaction_context)?;

    // Consume compute units if feature `native_programs_consume_cu` is activated
    let native_programs_consume_cu = invoke_context
        .feature_set
        .is_active(&native_programs_consume_cu::id());

    // Program Management Instruction
    if native_loader::check_id(program_account.get_owner()) {
        drop(program_account);
        let program_id = instruction_context.get_last_program_key(transaction_context)?;
        return if bpf_loader_upgradeable::check_id(program_id) {
            if native_programs_consume_cu {
                invoke_context.consume_checked(2_370)?;
            }
            process_loader_upgradeable_instruction(invoke_context, use_jit)
        } else if bpf_loader::check_id(program_id) {
            if native_programs_consume_cu {
                invoke_context.consume_checked(570)?;
            }
            process_loader_instruction(invoke_context, use_jit)
        } else if bpf_loader_deprecated::check_id(program_id) {
            if native_programs_consume_cu {
                invoke_context.consume_checked(1_140)?;
            }
            ic_logger_msg!(log_collector, "Deprecated loader is no longer supported");
            Err(InstructionError::UnsupportedProgramId)
        } else {
            ic_logger_msg!(log_collector, "Invalid BPF loader id");
            Err(InstructionError::IncorrectProgramId)
        };
    }

    // Program Invocation
    if !program_account.is_executable() {
        ic_logger_msg!(log_collector, "Program is not executable");
        return Err(InstructionError::IncorrectProgramId);
    }
    let programdata_account = if bpf_loader_upgradeable::check_id(program_account.get_owner()) {
        let programdata_account = instruction_context.try_borrow_program_account(
            transaction_context,
            instruction_context
                .get_number_of_program_accounts()
                .saturating_sub(2),
        )?;
        Some(programdata_account)
    } else {
        None
    };

    let mut get_or_create_executor_time = Measure::start("get_or_create_executor_time");
    let (executor, load_program_metrics) = load_program_from_account(
        &invoke_context.feature_set,
        invoke_context.get_compute_budget(),
        log_collector,
        Some(invoke_context.tx_executor_cache.borrow_mut()),
        &program_account,
        programdata_account.as_ref().unwrap_or(&program_account),
        use_jit,
    )?;
    drop(program_account);
    drop(programdata_account);
    get_or_create_executor_time.stop();
    saturating_add_assign!(
        invoke_context.timings.get_or_create_executor_us,
        get_or_create_executor_time.as_us()
    );
    if let Some(load_program_metrics) = load_program_metrics {
        load_program_metrics.submit_datapoint(&mut invoke_context.timings);
    }

    executor.usage_counter.fetch_add(1, Ordering::Relaxed);
    match &executor.program {
        LoadedProgramType::Invalid => Err(InstructionError::InvalidAccountData),
        LoadedProgramType::LegacyV0(executable) => execute(executable, invoke_context),
        LoadedProgramType::LegacyV1(executable) => execute(executable, invoke_context),
        _ => Err(InstructionError::IncorrectProgramId),
    }
}

fn process_loader_upgradeable_instruction(
    invoke_context: &mut InvokeContext,
    use_jit: bool,
) -> Result<(), InstructionError> {
    let log_collector = invoke_context.get_log_collector();
    let transaction_context = &invoke_context.transaction_context;
    let instruction_context = transaction_context.get_current_instruction_context()?;
    let instruction_data = instruction_context.get_instruction_data();
    let program_id = instruction_context.get_last_program_key(transaction_context)?;

    match limited_deserialize(instruction_data)? {
        UpgradeableLoaderInstruction::InitializeBuffer => {
            instruction_context.check_number_of_instruction_accounts(2)?;
            let mut buffer =
                instruction_context.try_borrow_instruction_account(transaction_context, 0)?;

            if UpgradeableLoaderState::Uninitialized != buffer.get_state()? {
                ic_logger_msg!(log_collector, "Buffer account already initialized");
                return Err(InstructionError::AccountAlreadyInitialized);
            }

            let authority_key = Some(*transaction_context.get_key_of_account_at_index(
                instruction_context.get_index_of_instruction_account_in_transaction(1)?,
            )?);

            buffer.set_state(&UpgradeableLoaderState::Buffer {
                authority_address: authority_key,
            })?;
        }
        UpgradeableLoaderInstruction::Write { offset, bytes } => {
            instruction_context.check_number_of_instruction_accounts(2)?;
            let buffer =
                instruction_context.try_borrow_instruction_account(transaction_context, 0)?;

            if let UpgradeableLoaderState::Buffer { authority_address } = buffer.get_state()? {
                if authority_address.is_none() {
                    ic_logger_msg!(log_collector, "Buffer is immutable");
                    return Err(InstructionError::Immutable); // TODO better error code
                }
                let authority_key = Some(*transaction_context.get_key_of_account_at_index(
                    instruction_context.get_index_of_instruction_account_in_transaction(1)?,
                )?);
                if authority_address != authority_key {
                    ic_logger_msg!(log_collector, "Incorrect buffer authority provided");
                    return Err(InstructionError::IncorrectAuthority);
                }
                if !instruction_context.is_instruction_account_signer(1)? {
                    ic_logger_msg!(log_collector, "Buffer authority did not sign");
                    return Err(InstructionError::MissingRequiredSignature);
                }
            } else {
                ic_logger_msg!(log_collector, "Invalid Buffer account");
                return Err(InstructionError::InvalidAccountData);
            }
            drop(buffer);
            write_program_data(
                UpgradeableLoaderState::size_of_buffer_metadata().saturating_add(offset as usize),
                &bytes,
                invoke_context,
            )?;
        }
        UpgradeableLoaderInstruction::DeployWithMaxDataLen { max_data_len } => {
            instruction_context.check_number_of_instruction_accounts(4)?;
            let payer_key = *transaction_context.get_key_of_account_at_index(
                instruction_context.get_index_of_instruction_account_in_transaction(0)?,
            )?;
            let programdata_key = *transaction_context.get_key_of_account_at_index(
                instruction_context.get_index_of_instruction_account_in_transaction(1)?,
            )?;
            let rent = get_sysvar_with_account_check::rent(invoke_context, instruction_context, 4)?;
            let clock =
                get_sysvar_with_account_check::clock(invoke_context, instruction_context, 5)?;
            instruction_context.check_number_of_instruction_accounts(8)?;
            let authority_key = Some(*transaction_context.get_key_of_account_at_index(
                instruction_context.get_index_of_instruction_account_in_transaction(7)?,
            )?);

            // Verify Program account

            let program =
                instruction_context.try_borrow_instruction_account(transaction_context, 2)?;
            if UpgradeableLoaderState::Uninitialized != program.get_state()? {
                ic_logger_msg!(log_collector, "Program account already initialized");
                return Err(InstructionError::AccountAlreadyInitialized);
            }
            if program.get_data().len() < UpgradeableLoaderState::size_of_program() {
                ic_logger_msg!(log_collector, "Program account too small");
                return Err(InstructionError::AccountDataTooSmall);
            }
            if program.get_lamports() < rent.minimum_balance(program.get_data().len()) {
                ic_logger_msg!(log_collector, "Program account not rent-exempt");
                return Err(InstructionError::ExecutableAccountNotRentExempt);
            }
            let new_program_id = *program.get_key();
            drop(program);

            // Verify Buffer account

            let buffer =
                instruction_context.try_borrow_instruction_account(transaction_context, 3)?;
            if let UpgradeableLoaderState::Buffer { authority_address } = buffer.get_state()? {
                if authority_address != authority_key {
                    ic_logger_msg!(log_collector, "Buffer and upgrade authority don't match");
                    return Err(InstructionError::IncorrectAuthority);
                }
                if !instruction_context.is_instruction_account_signer(7)? {
                    ic_logger_msg!(log_collector, "Upgrade authority did not sign");
                    return Err(InstructionError::MissingRequiredSignature);
                }
            } else {
                ic_logger_msg!(log_collector, "Invalid Buffer account");
                return Err(InstructionError::InvalidArgument);
            }
            let buffer_key = *buffer.get_key();
            let buffer_data_offset = UpgradeableLoaderState::size_of_buffer_metadata();
            let buffer_data_len = buffer.get_data().len().saturating_sub(buffer_data_offset);
            let programdata_data_offset = UpgradeableLoaderState::size_of_programdata_metadata();
            let programdata_len = UpgradeableLoaderState::size_of_programdata(max_data_len);
            if buffer.get_data().len() < UpgradeableLoaderState::size_of_buffer_metadata()
                || buffer_data_len == 0
            {
                ic_logger_msg!(log_collector, "Buffer account too small");
                return Err(InstructionError::InvalidAccountData);
            }
            drop(buffer);
            if max_data_len < buffer_data_len {
                ic_logger_msg!(
                    log_collector,
                    "Max data length is too small to hold Buffer data"
                );
                return Err(InstructionError::AccountDataTooSmall);
            }
            if programdata_len > MAX_PERMITTED_DATA_LENGTH as usize {
                ic_logger_msg!(log_collector, "Max data length is too large");
                return Err(InstructionError::InvalidArgument);
            }

            // Create ProgramData account
            let (derived_address, bump_seed) =
                Pubkey::find_program_address(&[new_program_id.as_ref()], program_id);
            if derived_address != programdata_key {
                ic_logger_msg!(log_collector, "ProgramData address is not derived");
                return Err(InstructionError::InvalidArgument);
            }

            // Drain the Buffer account to payer before paying for programdata account
            {
                let mut buffer =
                    instruction_context.try_borrow_instruction_account(transaction_context, 3)?;
                let mut payer =
                    instruction_context.try_borrow_instruction_account(transaction_context, 0)?;
                payer.checked_add_lamports(buffer.get_lamports())?;
                buffer.set_lamports(0)?;
            }

            let owner_id = *program_id;
            let mut instruction = system_instruction::create_account(
                &payer_key,
                &programdata_key,
                1.max(rent.minimum_balance(programdata_len)),
                programdata_len as u64,
                program_id,
            );

            // pass an extra account to avoid the overly strict UnbalancedInstruction error
            instruction
                .accounts
                .push(AccountMeta::new(buffer_key, false));

            let transaction_context = &invoke_context.transaction_context;
            let instruction_context = transaction_context.get_current_instruction_context()?;
            let caller_program_id =
                instruction_context.get_last_program_key(transaction_context)?;
            let signers = [[new_program_id.as_ref(), &[bump_seed]]]
                .iter()
                .map(|seeds| Pubkey::create_program_address(seeds, caller_program_id))
                .collect::<Result<Vec<Pubkey>, solana_sdk::pubkey::PubkeyError>>()?;
            invoke_context.native_invoke(instruction.into(), signers.as_slice())?;

            // Load and verify the program bits
            let transaction_context = &invoke_context.transaction_context;
            let instruction_context = transaction_context.get_current_instruction_context()?;
            let buffer =
                instruction_context.try_borrow_instruction_account(transaction_context, 3)?;
            deploy_program!(
                invoke_context,
                use_jit,
                new_program_id,
                &owner_id,
                UpgradeableLoaderState::size_of_program().saturating_add(programdata_len),
                clock.slot,
                {
                    drop(buffer);
                },
                buffer
                    .get_data()
                    .get(buffer_data_offset..)
                    .ok_or(InstructionError::AccountDataTooSmall)?,
            );

            let transaction_context = &invoke_context.transaction_context;
            let instruction_context = transaction_context.get_current_instruction_context()?;

            // Update the ProgramData account and record the program bits
            {
                let mut programdata =
                    instruction_context.try_borrow_instruction_account(transaction_context, 1)?;
                programdata.set_state(&UpgradeableLoaderState::ProgramData {
                    slot: clock.slot,
                    upgrade_authority_address: authority_key,
                })?;
                let dst_slice = programdata
                    .get_data_mut()?
                    .get_mut(
                        programdata_data_offset
                            ..programdata_data_offset.saturating_add(buffer_data_len),
                    )
                    .ok_or(InstructionError::AccountDataTooSmall)?;
                let mut buffer =
                    instruction_context.try_borrow_instruction_account(transaction_context, 3)?;
                let src_slice = buffer
                    .get_data()
                    .get(buffer_data_offset..)
                    .ok_or(InstructionError::AccountDataTooSmall)?;
                dst_slice.copy_from_slice(src_slice);
                if invoke_context
                    .feature_set
                    .is_active(&enable_program_redeployment_cooldown::id())
                {
                    buffer.set_data_length(UpgradeableLoaderState::size_of_buffer(0))?;
                }
            }

            // Update the Program account
            let mut program =
                instruction_context.try_borrow_instruction_account(transaction_context, 2)?;
            program.set_state(&UpgradeableLoaderState::Program {
                programdata_address: programdata_key,
            })?;
            program.set_executable(true)?;
            drop(program);

            ic_logger_msg!(log_collector, "Deployed program {:?}", new_program_id);
        }
        UpgradeableLoaderInstruction::Upgrade => {
            instruction_context.check_number_of_instruction_accounts(3)?;
            let programdata_key = *transaction_context.get_key_of_account_at_index(
                instruction_context.get_index_of_instruction_account_in_transaction(0)?,
            )?;
            let rent = get_sysvar_with_account_check::rent(invoke_context, instruction_context, 4)?;
            let clock =
                get_sysvar_with_account_check::clock(invoke_context, instruction_context, 5)?;
            instruction_context.check_number_of_instruction_accounts(7)?;
            let authority_key = Some(*transaction_context.get_key_of_account_at_index(
                instruction_context.get_index_of_instruction_account_in_transaction(6)?,
            )?);

            // Verify Program account

            let program =
                instruction_context.try_borrow_instruction_account(transaction_context, 1)?;
            if !program.is_executable() {
                ic_logger_msg!(log_collector, "Program account not executable");
                return Err(InstructionError::AccountNotExecutable);
            }
            if !program.is_writable() {
                ic_logger_msg!(log_collector, "Program account not writeable");
                return Err(InstructionError::InvalidArgument);
            }
            if program.get_owner() != program_id {
                ic_logger_msg!(log_collector, "Program account not owned by loader");
                return Err(InstructionError::IncorrectProgramId);
            }
            if let UpgradeableLoaderState::Program {
                programdata_address,
            } = program.get_state()?
            {
                if programdata_address != programdata_key {
                    ic_logger_msg!(log_collector, "Program and ProgramData account mismatch");
                    return Err(InstructionError::InvalidArgument);
                }
            } else {
                ic_logger_msg!(log_collector, "Invalid Program account");
                return Err(InstructionError::InvalidAccountData);
            }
            let new_program_id = *program.get_key();
            drop(program);

            // Verify Buffer account

            let buffer =
                instruction_context.try_borrow_instruction_account(transaction_context, 2)?;
            if let UpgradeableLoaderState::Buffer { authority_address } = buffer.get_state()? {
                if authority_address != authority_key {
                    ic_logger_msg!(log_collector, "Buffer and upgrade authority don't match");
                    return Err(InstructionError::IncorrectAuthority);
                }
                if !instruction_context.is_instruction_account_signer(6)? {
                    ic_logger_msg!(log_collector, "Upgrade authority did not sign");
                    return Err(InstructionError::MissingRequiredSignature);
                }
            } else {
                ic_logger_msg!(log_collector, "Invalid Buffer account");
                return Err(InstructionError::InvalidArgument);
            }
            let buffer_lamports = buffer.get_lamports();
            let buffer_data_offset = UpgradeableLoaderState::size_of_buffer_metadata();
            let buffer_data_len = buffer.get_data().len().saturating_sub(buffer_data_offset);
            if buffer.get_data().len() < UpgradeableLoaderState::size_of_buffer_metadata()
                || buffer_data_len == 0
            {
                ic_logger_msg!(log_collector, "Buffer account too small");
                return Err(InstructionError::InvalidAccountData);
            }
            drop(buffer);

            // Verify ProgramData account

            let programdata =
                instruction_context.try_borrow_instruction_account(transaction_context, 0)?;
            let programdata_data_offset = UpgradeableLoaderState::size_of_programdata_metadata();
            let programdata_balance_required =
                1.max(rent.minimum_balance(programdata.get_data().len()));
            if programdata.get_data().len()
                < UpgradeableLoaderState::size_of_programdata(buffer_data_len)
            {
                ic_logger_msg!(log_collector, "ProgramData account not large enough");
                return Err(InstructionError::AccountDataTooSmall);
            }
            if programdata.get_lamports().saturating_add(buffer_lamports)
                < programdata_balance_required
            {
                ic_logger_msg!(
                    log_collector,
                    "Buffer account balance too low to fund upgrade"
                );
                return Err(InstructionError::InsufficientFunds);
            }
            let deployment_slot = if let UpgradeableLoaderState::ProgramData {
                slot,
                upgrade_authority_address,
            } = programdata.get_state()?
            {
                if invoke_context
                    .feature_set
                    .is_active(&enable_program_redeployment_cooldown::id())
                    && clock.slot == slot
                {
                    ic_logger_msg!(log_collector, "Program was deployed in this block already");
                    return Err(InstructionError::InvalidArgument);
                }
                if upgrade_authority_address.is_none() {
                    ic_logger_msg!(log_collector, "Program not upgradeable");
                    return Err(InstructionError::Immutable);
                }
                if upgrade_authority_address != authority_key {
                    ic_logger_msg!(log_collector, "Incorrect upgrade authority provided");
                    return Err(InstructionError::IncorrectAuthority);
                }
                if !instruction_context.is_instruction_account_signer(6)? {
                    ic_logger_msg!(log_collector, "Upgrade authority did not sign");
                    return Err(InstructionError::MissingRequiredSignature);
                }
                slot
            } else {
                ic_logger_msg!(log_collector, "Invalid ProgramData account");
                return Err(InstructionError::InvalidAccountData);
            };
            let programdata_len = programdata.get_data().len();
            drop(programdata);

            // Load and verify the program bits
            let buffer =
                instruction_context.try_borrow_instruction_account(transaction_context, 2)?;
            deploy_program!(
                invoke_context,
                use_jit,
                new_program_id,
                program_id,
                UpgradeableLoaderState::size_of_program().saturating_add(programdata_len),
                deployment_slot,
                {
                    drop(buffer);
                },
                buffer
                    .get_data()
                    .get(buffer_data_offset..)
                    .ok_or(InstructionError::AccountDataTooSmall)?,
            );

            let transaction_context = &invoke_context.transaction_context;
            let instruction_context = transaction_context.get_current_instruction_context()?;

            // Update the ProgramData account, record the upgraded data, and zero
            // the rest
            let mut programdata =
                instruction_context.try_borrow_instruction_account(transaction_context, 0)?;
            {
                programdata.set_state(&UpgradeableLoaderState::ProgramData {
                    slot: clock.slot,
                    upgrade_authority_address: authority_key,
                })?;
                let dst_slice = programdata
                    .get_data_mut()?
                    .get_mut(
                        programdata_data_offset
                            ..programdata_data_offset.saturating_add(buffer_data_len),
                    )
                    .ok_or(InstructionError::AccountDataTooSmall)?;
                let buffer =
                    instruction_context.try_borrow_instruction_account(transaction_context, 2)?;
                let src_slice = buffer
                    .get_data()
                    .get(buffer_data_offset..)
                    .ok_or(InstructionError::AccountDataTooSmall)?;
                dst_slice.copy_from_slice(src_slice);
            }
            programdata
                .get_data_mut()?
                .get_mut(programdata_data_offset.saturating_add(buffer_data_len)..)
                .ok_or(InstructionError::AccountDataTooSmall)?
                .fill(0);

            // Fund ProgramData to rent-exemption, spill the rest
            let mut buffer =
                instruction_context.try_borrow_instruction_account(transaction_context, 2)?;
            let mut spill =
                instruction_context.try_borrow_instruction_account(transaction_context, 3)?;
            spill.checked_add_lamports(
                programdata
                    .get_lamports()
                    .saturating_add(buffer_lamports)
                    .saturating_sub(programdata_balance_required),
            )?;
            buffer.set_lamports(0)?;
            programdata.set_lamports(programdata_balance_required)?;
            if invoke_context
                .feature_set
                .is_active(&enable_program_redeployment_cooldown::id())
            {
                buffer.set_data_length(UpgradeableLoaderState::size_of_buffer(0))?;
            }

            ic_logger_msg!(log_collector, "Upgraded program {:?}", new_program_id);
        }
        UpgradeableLoaderInstruction::SetAuthority => {
            instruction_context.check_number_of_instruction_accounts(2)?;
            let mut account =
                instruction_context.try_borrow_instruction_account(transaction_context, 0)?;
            let present_authority_key = transaction_context.get_key_of_account_at_index(
                instruction_context.get_index_of_instruction_account_in_transaction(1)?,
            )?;
            let new_authority = instruction_context
                .get_index_of_instruction_account_in_transaction(2)
                .and_then(|index_in_transaction| {
                    transaction_context.get_key_of_account_at_index(index_in_transaction)
                })
                .ok();

            match account.get_state()? {
                UpgradeableLoaderState::Buffer { authority_address } => {
                    if new_authority.is_none() {
                        ic_logger_msg!(log_collector, "Buffer authority is not optional");
                        return Err(InstructionError::IncorrectAuthority);
                    }
                    if authority_address.is_none() {
                        ic_logger_msg!(log_collector, "Buffer is immutable");
                        return Err(InstructionError::Immutable);
                    }
                    if authority_address != Some(*present_authority_key) {
                        ic_logger_msg!(log_collector, "Incorrect buffer authority provided");
                        return Err(InstructionError::IncorrectAuthority);
                    }
                    if !instruction_context.is_instruction_account_signer(1)? {
                        ic_logger_msg!(log_collector, "Buffer authority did not sign");
                        return Err(InstructionError::MissingRequiredSignature);
                    }
                    account.set_state(&UpgradeableLoaderState::Buffer {
                        authority_address: new_authority.cloned(),
                    })?;
                }
                UpgradeableLoaderState::ProgramData {
                    slot,
                    upgrade_authority_address,
                } => {
                    if upgrade_authority_address.is_none() {
                        ic_logger_msg!(log_collector, "Program not upgradeable");
                        return Err(InstructionError::Immutable);
                    }
                    if upgrade_authority_address != Some(*present_authority_key) {
                        ic_logger_msg!(log_collector, "Incorrect upgrade authority provided");
                        return Err(InstructionError::IncorrectAuthority);
                    }
                    if !instruction_context.is_instruction_account_signer(1)? {
                        ic_logger_msg!(log_collector, "Upgrade authority did not sign");
                        return Err(InstructionError::MissingRequiredSignature);
                    }
                    account.set_state(&UpgradeableLoaderState::ProgramData {
                        slot,
                        upgrade_authority_address: new_authority.cloned(),
                    })?;
                }
                _ => {
                    ic_logger_msg!(log_collector, "Account does not support authorities");
                    return Err(InstructionError::InvalidArgument);
                }
            }

            ic_logger_msg!(log_collector, "New authority {:?}", new_authority);
        }
        UpgradeableLoaderInstruction::SetAuthorityChecked => {
            if !invoke_context
                .feature_set
                .is_active(&enable_bpf_loader_set_authority_checked_ix::id())
            {
                return Err(InstructionError::InvalidInstructionData);
            }

            instruction_context.check_number_of_instruction_accounts(3)?;
            let mut account =
                instruction_context.try_borrow_instruction_account(transaction_context, 0)?;
            let present_authority_key = transaction_context.get_key_of_account_at_index(
                instruction_context.get_index_of_instruction_account_in_transaction(1)?,
            )?;
            let new_authority_key = transaction_context.get_key_of_account_at_index(
                instruction_context.get_index_of_instruction_account_in_transaction(2)?,
            )?;

            match account.get_state()? {
                UpgradeableLoaderState::Buffer { authority_address } => {
                    if authority_address.is_none() {
                        ic_logger_msg!(log_collector, "Buffer is immutable");
                        return Err(InstructionError::Immutable);
                    }
                    if authority_address != Some(*present_authority_key) {
                        ic_logger_msg!(log_collector, "Incorrect buffer authority provided");
                        return Err(InstructionError::IncorrectAuthority);
                    }
                    if !instruction_context.is_instruction_account_signer(1)? {
                        ic_logger_msg!(log_collector, "Buffer authority did not sign");
                        return Err(InstructionError::MissingRequiredSignature);
                    }
                    if !instruction_context.is_instruction_account_signer(2)? {
                        ic_logger_msg!(log_collector, "New authority did not sign");
                        return Err(InstructionError::MissingRequiredSignature);
                    }
                    account.set_state(&UpgradeableLoaderState::Buffer {
                        authority_address: Some(*new_authority_key),
                    })?;
                }
                UpgradeableLoaderState::ProgramData {
                    slot,
                    upgrade_authority_address,
                } => {
                    if upgrade_authority_address.is_none() {
                        ic_logger_msg!(log_collector, "Program not upgradeable");
                        return Err(InstructionError::Immutable);
                    }
                    if upgrade_authority_address != Some(*present_authority_key) {
                        ic_logger_msg!(log_collector, "Incorrect upgrade authority provided");
                        return Err(InstructionError::IncorrectAuthority);
                    }
                    if !instruction_context.is_instruction_account_signer(1)? {
                        ic_logger_msg!(log_collector, "Upgrade authority did not sign");
                        return Err(InstructionError::MissingRequiredSignature);
                    }
                    if !instruction_context.is_instruction_account_signer(2)? {
                        ic_logger_msg!(log_collector, "New authority did not sign");
                        return Err(InstructionError::MissingRequiredSignature);
                    }
                    account.set_state(&UpgradeableLoaderState::ProgramData {
                        slot,
                        upgrade_authority_address: Some(*new_authority_key),
                    })?;
                }
                _ => {
                    ic_logger_msg!(log_collector, "Account does not support authorities");
                    return Err(InstructionError::InvalidArgument);
                }
            }

            ic_logger_msg!(log_collector, "New authority {:?}", new_authority_key);
        }
        UpgradeableLoaderInstruction::Close => {
            instruction_context.check_number_of_instruction_accounts(2)?;
            if instruction_context.get_index_of_instruction_account_in_transaction(0)?
                == instruction_context.get_index_of_instruction_account_in_transaction(1)?
            {
                ic_logger_msg!(
                    log_collector,
                    "Recipient is the same as the account being closed"
                );
                return Err(InstructionError::InvalidArgument);
            }
            let mut close_account =
                instruction_context.try_borrow_instruction_account(transaction_context, 0)?;
            let close_key = *close_account.get_key();
            let close_account_state = close_account.get_state()?;
            if invoke_context
                .feature_set
                .is_active(&enable_program_redeployment_cooldown::id())
            {
                close_account.set_data_length(UpgradeableLoaderState::size_of_uninitialized())?;
            }
            match close_account_state {
                UpgradeableLoaderState::Uninitialized => {
                    let mut recipient_account = instruction_context
                        .try_borrow_instruction_account(transaction_context, 1)?;
                    recipient_account.checked_add_lamports(close_account.get_lamports())?;
                    close_account.set_lamports(0)?;

                    ic_logger_msg!(log_collector, "Closed Uninitialized {}", close_key);
                }
                UpgradeableLoaderState::Buffer { authority_address } => {
                    instruction_context.check_number_of_instruction_accounts(3)?;
                    drop(close_account);
                    common_close_account(
                        &authority_address,
                        transaction_context,
                        instruction_context,
                        &log_collector,
                    )?;

                    ic_logger_msg!(log_collector, "Closed Buffer {}", close_key);
                }
                UpgradeableLoaderState::ProgramData {
                    slot,
                    upgrade_authority_address: authority_address,
                } => {
                    instruction_context.check_number_of_instruction_accounts(4)?;
                    drop(close_account);
                    let program_account = instruction_context
                        .try_borrow_instruction_account(transaction_context, 3)?;
                    let program_key = *program_account.get_key();

                    if !program_account.is_writable() {
                        ic_logger_msg!(log_collector, "Program account is not writable");
                        return Err(InstructionError::InvalidArgument);
                    }
                    if program_account.get_owner() != program_id {
                        ic_logger_msg!(log_collector, "Program account not owned by loader");
                        return Err(InstructionError::IncorrectProgramId);
                    }
                    if invoke_context
                        .feature_set
                        .is_active(&enable_program_redeployment_cooldown::id())
                    {
                        let clock = invoke_context.get_sysvar_cache().get_clock()?;
                        if clock.slot == slot {
                            ic_logger_msg!(
                                log_collector,
                                "Program was deployed in this block already"
                            );
                            return Err(InstructionError::InvalidArgument);
                        }
                    }

                    match program_account.get_state()? {
                        UpgradeableLoaderState::Program {
                            programdata_address,
                        } => {
                            if programdata_address != close_key {
                                ic_logger_msg!(
                                    log_collector,
                                    "ProgramData account does not match ProgramData account"
                                );
                                return Err(InstructionError::InvalidArgument);
                            }

                            drop(program_account);
                            common_close_account(
                                &authority_address,
                                transaction_context,
                                instruction_context,
                                &log_collector,
                            )?;
                            if invoke_context
                                .feature_set
                                .is_active(&delay_visibility_of_program_deployment::id())
                            {
                                let clock = invoke_context.get_sysvar_cache().get_clock()?;
                                invoke_context
                                    .tx_executor_cache
                                    .borrow_mut()
                                    .set_tombstone(program_key, clock.slot);
                            }
                        }
                        _ => {
                            ic_logger_msg!(log_collector, "Invalid Program account");
                            return Err(InstructionError::InvalidArgument);
                        }
                    }

                    ic_logger_msg!(log_collector, "Closed Program {}", program_key);
                }
                _ => {
                    ic_logger_msg!(log_collector, "Account does not support closing");
                    return Err(InstructionError::InvalidArgument);
                }
            }
        }
        UpgradeableLoaderInstruction::ExtendProgram { additional_bytes } => {
            if !invoke_context
                .feature_set
                .is_active(&enable_bpf_loader_extend_program_ix::ID)
            {
                return Err(InstructionError::InvalidInstructionData);
            }

            if additional_bytes == 0 {
                ic_logger_msg!(log_collector, "Additional bytes must be greater than 0");
                return Err(InstructionError::InvalidInstructionData);
            }

            const PROGRAM_DATA_ACCOUNT_INDEX: IndexOfAccount = 0;
            const PROGRAM_ACCOUNT_INDEX: IndexOfAccount = 1;
            #[allow(dead_code)]
            // System program is only required when a CPI is performed
            const OPTIONAL_SYSTEM_PROGRAM_ACCOUNT_INDEX: IndexOfAccount = 2;
            const OPTIONAL_PAYER_ACCOUNT_INDEX: IndexOfAccount = 3;

            let programdata_account = instruction_context
                .try_borrow_instruction_account(transaction_context, PROGRAM_DATA_ACCOUNT_INDEX)?;
            let programdata_key = *programdata_account.get_key();

            if program_id != programdata_account.get_owner() {
                ic_logger_msg!(log_collector, "ProgramData owner is invalid");
                return Err(InstructionError::InvalidAccountOwner);
            }
            if !programdata_account.is_writable() {
                ic_logger_msg!(log_collector, "ProgramData is not writable");
                return Err(InstructionError::InvalidArgument);
            }

            let program_account = instruction_context
                .try_borrow_instruction_account(transaction_context, PROGRAM_ACCOUNT_INDEX)?;
            if !program_account.is_writable() {
                ic_logger_msg!(log_collector, "Program account is not writable");
                return Err(InstructionError::InvalidArgument);
            }
            if program_account.get_owner() != program_id {
                ic_logger_msg!(log_collector, "Program account not owned by loader");
                return Err(InstructionError::InvalidAccountOwner);
            }
            match program_account.get_state()? {
                UpgradeableLoaderState::Program {
                    programdata_address,
                } => {
                    if programdata_address != programdata_key {
                        ic_logger_msg!(
                            log_collector,
                            "Program account does not match ProgramData account"
                        );
                        return Err(InstructionError::InvalidArgument);
                    }
                }
                _ => {
                    ic_logger_msg!(log_collector, "Invalid Program account");
                    return Err(InstructionError::InvalidAccountData);
                }
            }
            drop(program_account);

            let old_len = programdata_account.get_data().len();
            let new_len = old_len.saturating_add(additional_bytes as usize);
            if new_len > MAX_PERMITTED_DATA_LENGTH as usize {
                ic_logger_msg!(
                    log_collector,
                    "Extended ProgramData length of {} bytes exceeds max account data length of {} bytes",
                    new_len,
                    MAX_PERMITTED_DATA_LENGTH
                );
                return Err(InstructionError::InvalidRealloc);
            }

            if let UpgradeableLoaderState::ProgramData {
                slot: _,
                upgrade_authority_address,
            } = programdata_account.get_state()?
            {
                if upgrade_authority_address.is_none() {
                    ic_logger_msg!(
                        log_collector,
                        "Cannot extend ProgramData accounts that are not upgradeable"
                    );
                    return Err(InstructionError::Immutable);
                }
            } else {
                ic_logger_msg!(log_collector, "ProgramData state is invalid");
                return Err(InstructionError::InvalidAccountData);
            }

            let required_payment = {
                let balance = programdata_account.get_lamports();
                let rent = invoke_context.get_sysvar_cache().get_rent()?;
                let min_balance = rent.minimum_balance(new_len).max(1);
                min_balance.saturating_sub(balance)
            };

            // Borrowed accounts need to be dropped before native_invoke
            drop(programdata_account);

            if required_payment > 0 {
                let payer_key = *transaction_context.get_key_of_account_at_index(
                    instruction_context.get_index_of_instruction_account_in_transaction(
                        OPTIONAL_PAYER_ACCOUNT_INDEX,
                    )?,
                )?;

                invoke_context.native_invoke(
                    system_instruction::transfer(&payer_key, &programdata_key, required_payment)
                        .into(),
                    &[],
                )?;
            }

            let transaction_context = &invoke_context.transaction_context;
            let instruction_context = transaction_context.get_current_instruction_context()?;
            let mut programdata_account = instruction_context
                .try_borrow_instruction_account(transaction_context, PROGRAM_DATA_ACCOUNT_INDEX)?;
            programdata_account.set_data_length(new_len)?;

            ic_logger_msg!(
                log_collector,
                "Extended ProgramData account by {} bytes",
                additional_bytes
            );
        }
    }

    Ok(())
}

fn common_close_account(
    authority_address: &Option<Pubkey>,
    transaction_context: &TransactionContext,
    instruction_context: &InstructionContext,
    log_collector: &Option<Rc<RefCell<LogCollector>>>,
) -> Result<(), InstructionError> {
    if authority_address.is_none() {
        ic_logger_msg!(log_collector, "Account is immutable");
        return Err(InstructionError::Immutable);
    }
    if *authority_address
        != Some(*transaction_context.get_key_of_account_at_index(
            instruction_context.get_index_of_instruction_account_in_transaction(2)?,
        )?)
    {
        ic_logger_msg!(log_collector, "Incorrect authority provided");
        return Err(InstructionError::IncorrectAuthority);
    }
    if !instruction_context.is_instruction_account_signer(2)? {
        ic_logger_msg!(log_collector, "Authority did not sign");
        return Err(InstructionError::MissingRequiredSignature);
    }

    let mut close_account =
        instruction_context.try_borrow_instruction_account(transaction_context, 0)?;
    let mut recipient_account =
        instruction_context.try_borrow_instruction_account(transaction_context, 1)?;
    recipient_account.checked_add_lamports(close_account.get_lamports())?;
    close_account.set_lamports(0)?;
    close_account.set_state(&UpgradeableLoaderState::Uninitialized)?;
    Ok(())
}

fn process_loader_instruction(
    invoke_context: &mut InvokeContext,
    use_jit: bool,
) -> Result<(), InstructionError> {
    let transaction_context = &invoke_context.transaction_context;
    let instruction_context = transaction_context.get_current_instruction_context()?;
    let instruction_data = instruction_context.get_instruction_data();
    let program_id = instruction_context.get_last_program_key(transaction_context)?;
    let mut program = instruction_context.try_borrow_instruction_account(transaction_context, 0)?;
    if program.get_owner() != program_id {
        ic_msg!(
            invoke_context,
            "Executable account not owned by the BPF loader"
        );
        return Err(InstructionError::IncorrectProgramId);
    }
    let is_program_signer = program.is_signer();
    match limited_deserialize(instruction_data)? {
        LoaderInstruction::Write { offset, bytes } => {
            if !is_program_signer {
                ic_msg!(invoke_context, "Program account did not sign");
                return Err(InstructionError::MissingRequiredSignature);
            }
            drop(program);
            write_program_data(offset as usize, &bytes, invoke_context)?;
        }
        LoaderInstruction::Finalize => {
            if !is_program_signer {
                ic_msg!(invoke_context, "key[0] did not sign the transaction");
                return Err(InstructionError::MissingRequiredSignature);
            }
            deploy_program!(
                invoke_context,
                use_jit,
                *program.get_key(),
                program.get_owner(),
                program.get_data().len(),
                0,
                {},
                program.get_data(),
            );
            program.set_executable(true)?;
            ic_msg!(invoke_context, "Finalized account {:?}", program.get_key());
        }
    }

    Ok(())
}

fn execute<'a, 'b: 'a>(
    executable: &'a VerifiedExecutable<RequisiteVerifier, InvokeContext<'static>>,
    invoke_context: &'a mut InvokeContext<'b>,
) -> Result<(), InstructionError> {
    let log_collector = invoke_context.get_log_collector();
    let stack_height = invoke_context.get_stack_height();
    let transaction_context = &invoke_context.transaction_context;
    let instruction_context = transaction_context.get_current_instruction_context()?;
    let program_id = *instruction_context.get_last_program_key(transaction_context)?;
    #[cfg(any(target_os = "windows", not(target_arch = "x86_64")))]
    let use_jit = false;
    #[cfg(all(not(target_os = "windows"), target_arch = "x86_64"))]
    let use_jit = executable.get_executable().get_compiled_program().is_some();

    let mut serialize_time = Measure::start("serialize");
    let (parameter_bytes, regions, account_lengths) = serialize_parameters(
        invoke_context.transaction_context,
        instruction_context,
        invoke_context
            .feature_set
            .is_active(&cap_bpf_program_instruction_accounts::ID),
    )?;
    serialize_time.stop();

    let mut create_vm_time = Measure::start("create_vm");
    let mut execute_time;
    let execution_result = {
        let compute_meter_prev = invoke_context.get_remaining();
        create_vm!(
            vm,
            // We dropped the lifetime tracking in the Executor by setting it to 'static,
            // thus we need to reintroduce the correct lifetime of InvokeContext here again.
            unsafe {
                mem::transmute::<_, &'a VerifiedExecutable<RequisiteVerifier, InvokeContext<'b>>>(
                    executable,
                )
            },
            stack,
            heap,
            regions,
            account_lengths,
            invoke_context
        );
        let mut vm = match vm {
            Ok(info) => info,
            Err(e) => {
                ic_logger_msg!(log_collector, "Failed to create SBF VM: {}", e);
                return Err(InstructionError::ProgramEnvironmentSetupFailure);
            }
        };
        create_vm_time.stop();

        execute_time = Measure::start("execute");
        stable_log::program_invoke(&log_collector, &program_id, stack_height);
        let (compute_units_consumed, result) = vm.execute_program(!use_jit);
        drop(vm);
        ic_logger_msg!(
            log_collector,
            "Program {} consumed {} of {} compute units",
            &program_id,
            compute_units_consumed,
            compute_meter_prev
        );
        let (_returned_from_program_id, return_data) =
            invoke_context.transaction_context.get_return_data();
        if !return_data.is_empty() {
            stable_log::program_return(&log_collector, &program_id, return_data);
        }
        match result {
            ProgramResult::Ok(status) if status != SUCCESS => {
                let error: InstructionError = if (status == MAX_ACCOUNTS_DATA_ALLOCATIONS_EXCEEDED
                    && !invoke_context
                        .feature_set
                        .is_active(&cap_accounts_data_allocations_per_transaction::id()))
                    || (status == MAX_INSTRUCTION_TRACE_LENGTH_EXCEEDED
                        && !invoke_context
                            .feature_set
                            .is_active(&limit_max_instruction_trace_length::id()))
                {
                    // Until the cap_accounts_data_allocations_per_transaction feature is
                    // enabled, map the `MAX_ACCOUNTS_DATA_ALLOCATIONS_EXCEEDED` error to `InvalidError`.
                    // Until the limit_max_instruction_trace_length feature is
                    // enabled, map the `MAX_INSTRUCTION_TRACE_LENGTH_EXCEEDED` error to `InvalidError`.
                    InstructionError::InvalidError
                } else {
                    status.into()
                };
                stable_log::program_failure(&log_collector, &program_id, &error);
                Err(error)
            }
            ProgramResult::Err(error) => {
                let error = match error {
                    /*EbpfError::UserError(user_error) if let BpfError::SyscallError(
                        SyscallError::InstructionError(instruction_error),
                    ) = user_error.downcast_ref::<BpfError>().unwrap() => instruction_error.clone(),*/
                    EbpfError::UserError(user_error)
                        if matches!(
                            user_error.downcast_ref::<BpfError>().unwrap(),
                            BpfError::SyscallError(SyscallError::InstructionError(_)),
                        ) =>
                    {
                        match user_error.downcast_ref::<BpfError>().unwrap() {
                            BpfError::SyscallError(SyscallError::InstructionError(
                                instruction_error,
                            )) => instruction_error.clone(),
                            _ => unreachable!(),
                        }
                    }
                    err => {
                        ic_logger_msg!(log_collector, "Program failed to complete: {}", err);
                        InstructionError::ProgramFailedToComplete
                    }
                };
                stable_log::program_failure(&log_collector, &program_id, &error);
                Err(error)
            }
            _ => Ok(()),
        }
    };
    execute_time.stop();

    let mut deserialize_time = Measure::start("deserialize");
    let execute_or_deserialize_result = execution_result.and_then(|_| {
        deserialize_parameters(
            invoke_context.transaction_context,
            invoke_context
                .transaction_context
                .get_current_instruction_context()?,
            parameter_bytes.as_slice(),
            invoke_context.get_orig_account_lengths()?,
        )
    });
    deserialize_time.stop();

    // Update the timings
    let timings = &mut invoke_context.timings;
    timings.serialize_us = timings.serialize_us.saturating_add(serialize_time.as_us());
    timings.create_vm_us = timings.create_vm_us.saturating_add(create_vm_time.as_us());
    timings.execute_us = timings.execute_us.saturating_add(execute_time.as_us());
    timings.deserialize_us = timings
        .deserialize_us
        .saturating_add(deserialize_time.as_us());

    if execute_or_deserialize_result.is_ok() {
        stable_log::program_success(&log_collector, &program_id);
    }
    execute_or_deserialize_result
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        rand::Rng,
        solana_program_runtime::invoke_context::mock_process_instruction,
        solana_rbpf::{
            ebpf::MM_INPUT_START,
            elf::Executable,
            verifier::Verifier,
            vm::{BuiltInProgram, Config, ContextObject, FunctionRegistry},
        },
        solana_sdk::{
            account::{
                create_account_shared_data_for_test as create_account_for_test, AccountSharedData,
                ReadableAccount, WritableAccount,
            },
            account_utils::StateMut,
            clock::Clock,
            instruction::{AccountMeta, InstructionError},
            pubkey::Pubkey,
            rent::Rent,
            system_program, sysvar,
        },
        std::{fs::File, io::Read, ops::Range},
    };

    struct TestContextObject {
        remaining: u64,
    }
    impl ContextObject for TestContextObject {
        fn trace(&mut self, _state: [u64; 12]) {}
        fn consume(&mut self, amount: u64) {
            self.remaining = self.remaining.saturating_sub(amount);
        }
        fn get_remaining(&self) -> u64 {
            self.remaining
        }
    }

    fn process_instruction(
        loader_id: &Pubkey,
        program_indices: &[IndexOfAccount],
        instruction_data: &[u8],
        transaction_accounts: Vec<(Pubkey, AccountSharedData)>,
        instruction_accounts: Vec<AccountMeta>,
        expected_result: Result<(), InstructionError>,
    ) -> Vec<AccountSharedData> {
        mock_process_instruction(
            loader_id,
            program_indices.to_vec(),
            instruction_data,
            transaction_accounts,
            instruction_accounts,
            None,
            None,
            expected_result,
            super::process_instruction,
        )
    }

    fn load_program_account_from_elf(loader_id: &Pubkey, path: &str) -> AccountSharedData {
        let mut file = File::open(path).expect("file open failed");
        let mut elf = Vec::new();
        file.read_to_end(&mut elf).unwrap();
        let rent = Rent::default();
        let mut program_account =
            AccountSharedData::new(rent.minimum_balance(elf.len()), 0, loader_id);
        program_account.set_data(elf);
        program_account.set_executable(true);
        program_account
    }

    struct TautologyVerifier {}
    impl Verifier for TautologyVerifier {
        fn verify(
            _prog: &[u8],
            _config: &Config,
            _function_registry: &FunctionRegistry,
        ) -> std::result::Result<(), VerifierError> {
            Ok(())
        }
    }

    #[test]
    #[should_panic(expected = "ExceededMaxInstructions(31, 10)")]
    fn test_bpf_loader_non_terminating_program() {
        #[rustfmt::skip]
        let program = &[
            0x07, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, // r6 + 1
            0x05, 0x00, 0xfe, 0xff, 0x00, 0x00, 0x00, 0x00, // goto -2
            0x95, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // exit
        ];
        let mut input_mem = [0x00];
        let bpf_functions = std::collections::BTreeMap::<u32, (usize, String)>::new();
        let executable = Executable::<TestContextObject>::from_text_bytes(
            program,
            Arc::new(BuiltInProgram::new_loader(Config::default())),
            bpf_functions,
        )
        .unwrap();
        let verified_executable =
            VerifiedExecutable::<TautologyVerifier, TestContextObject>::from_executable(executable)
                .unwrap();
        let input_region = MemoryRegion::new_writable(&mut input_mem, MM_INPUT_START);
        let mut context_object = TestContextObject { remaining: 10 };
        let mut stack = AlignedMemory::zero_filled(
            verified_executable
                .get_executable()
                .get_config()
                .stack_size(),
        );
        let mut heap = AlignedMemory::with_capacity(0);
        let stack_len = stack.len();

        let memory_mapping = create_memory_mapping(
            verified_executable.get_executable(),
            &mut stack,
            &mut heap,
            vec![input_region],
            None,
        )
        .unwrap();
        let mut vm = EbpfVm::new(
            &verified_executable,
            &mut context_object,
            memory_mapping,
            stack_len,
        )
        .unwrap();
        vm.execute_program(true).1.unwrap();
    }

    #[test]
    #[should_panic(expected = "LDDWCannotBeLast")]
    fn test_bpf_loader_check_load_dw() {
        let prog = &[
            0x18, 0x00, 0x00, 0x00, 0x88, 0x77, 0x66, 0x55, // first half of lddw
        ];
        RequisiteVerifier::verify(prog, &Config::default(), &FunctionRegistry::default()).unwrap();
    }

    #[test]
    fn test_bpf_loader_write() {
        let loader_id = bpf_loader::id();
        let program_id = Pubkey::new_unique();
        let mut program_account = AccountSharedData::new(1, 0, &loader_id);
        let instruction_data = bincode::serialize(&LoaderInstruction::Write {
            offset: 3,
            bytes: vec![1, 2, 3],
        })
        .unwrap();

        // Case: No program account
        process_instruction(
            &loader_id,
            &[],
            &instruction_data,
            Vec::new(),
            Vec::new(),
            Err(InstructionError::NotEnoughAccountKeys),
        );

        // Case: Not signed
        process_instruction(
            &loader_id,
            &[],
            &instruction_data,
            vec![(program_id, program_account.clone())],
            vec![AccountMeta {
                pubkey: program_id,
                is_signer: false,
                is_writable: true,
            }],
            Err(InstructionError::MissingRequiredSignature),
        );

        // Case: Write bytes to an offset
        program_account.set_data(vec![0; 6]);
        let accounts = process_instruction(
            &loader_id,
            &[],
            &instruction_data,
            vec![(program_id, program_account.clone())],
            vec![AccountMeta {
                pubkey: program_id,
                is_signer: true,
                is_writable: true,
            }],
            Ok(()),
        );
        assert_eq!(&vec![0, 0, 0, 1, 2, 3], accounts.first().unwrap().data());

        // Case: Overflow
        program_account.set_data(vec![0; 5]);
        process_instruction(
            &loader_id,
            &[],
            &instruction_data,
            vec![(program_id, program_account)],
            vec![AccountMeta {
                pubkey: program_id,
                is_signer: true,
                is_writable: true,
            }],
            Err(InstructionError::AccountDataTooSmall),
        );
    }

    #[test]
    fn test_bpf_loader_finalize() {
        let loader_id = bpf_loader::id();
        let program_id = Pubkey::new_unique();
        let mut program_account =
            load_program_account_from_elf(&loader_id, "test_elfs/out/noop_aligned.so");
        program_account.set_executable(false);
        let instruction_data = bincode::serialize(&LoaderInstruction::Finalize).unwrap();

        // Case: No program account
        process_instruction(
            &loader_id,
            &[],
            &instruction_data,
            Vec::new(),
            Vec::new(),
            Err(InstructionError::NotEnoughAccountKeys),
        );

        // Case: Not signed
        process_instruction(
            &loader_id,
            &[],
            &instruction_data,
            vec![(program_id, program_account.clone())],
            vec![AccountMeta {
                pubkey: program_id,
                is_signer: false,
                is_writable: true,
            }],
            Err(InstructionError::MissingRequiredSignature),
        );

        // Case: Finalize
        let accounts = process_instruction(
            &loader_id,
            &[],
            &instruction_data,
            vec![(program_id, program_account.clone())],
            vec![AccountMeta {
                pubkey: program_id,
                is_signer: true,
                is_writable: true,
            }],
            Ok(()),
        );
        assert!(accounts.first().unwrap().executable());

        // Case: Finalize bad ELF
        *program_account.data_as_mut_slice().get_mut(0).unwrap() = 0;
        process_instruction(
            &loader_id,
            &[],
            &instruction_data,
            vec![(program_id, program_account)],
            vec![AccountMeta {
                pubkey: program_id,
                is_signer: true,
                is_writable: true,
            }],
            Err(InstructionError::InvalidAccountData),
        );
    }

    #[test]
    fn test_bpf_loader_invoke_main() {
        let loader_id = bpf_loader::id();
        let program_id = Pubkey::new_unique();
        let mut program_account =
            load_program_account_from_elf(&loader_id, "test_elfs/out/noop_aligned.so");
        let parameter_id = Pubkey::new_unique();
        let parameter_account = AccountSharedData::new(1, 0, &loader_id);
        let parameter_meta = AccountMeta {
            pubkey: parameter_id,
            is_signer: false,
            is_writable: false,
        };

        // Case: No program account
        process_instruction(
            &loader_id,
            &[],
            &[],
            Vec::new(),
            Vec::new(),
            Err(InstructionError::NotEnoughAccountKeys),
        );

        // Case: Only a program account
        process_instruction(
            &loader_id,
            &[0],
            &[],
            vec![(program_id, program_account.clone())],
            Vec::new(),
            Ok(()),
        );

        // Case: With program and parameter account
        process_instruction(
            &loader_id,
            &[0],
            &[],
            vec![
                (program_id, program_account.clone()),
                (parameter_id, parameter_account.clone()),
            ],
            vec![parameter_meta.clone()],
            Ok(()),
        );

        // Case: With duplicate accounts
        process_instruction(
            &loader_id,
            &[0],
            &[],
            vec![
                (program_id, program_account.clone()),
                (parameter_id, parameter_account),
            ],
            vec![parameter_meta.clone(), parameter_meta],
            Ok(()),
        );

        // Case: limited budget
        mock_process_instruction(
            &loader_id,
            vec![0],
            &[],
            vec![(program_id, program_account.clone())],
            Vec::new(),
            None,
            None,
            Err(InstructionError::ProgramFailedToComplete),
            |invoke_context: &mut InvokeContext| {
                invoke_context.mock_set_remaining(0);
                super::process_instruction(invoke_context)
            },
        );

        // Case: Account not a program
        program_account.set_executable(false);
        process_instruction(
            &loader_id,
            &[0],
            &[],
            vec![(program_id, program_account)],
            Vec::new(),
            Err(InstructionError::IncorrectProgramId),
        );
    }

    #[test]
    fn test_bpf_loader_serialize_unaligned() {
        let loader_id = bpf_loader_deprecated::id();
        let program_id = Pubkey::new_unique();
        let program_account =
            load_program_account_from_elf(&loader_id, "test_elfs/out/noop_unaligned.so");
        let parameter_id = Pubkey::new_unique();
        let parameter_account = AccountSharedData::new(1, 0, &loader_id);
        let parameter_meta = AccountMeta {
            pubkey: parameter_id,
            is_signer: false,
            is_writable: false,
        };

        // Case: With program and parameter account
        process_instruction(
            &loader_id,
            &[0],
            &[],
            vec![
                (program_id, program_account.clone()),
                (parameter_id, parameter_account.clone()),
            ],
            vec![parameter_meta.clone()],
            Ok(()),
        );

        // Case: With duplicate accounts
        process_instruction(
            &loader_id,
            &[0],
            &[],
            vec![
                (program_id, program_account),
                (parameter_id, parameter_account),
            ],
            vec![parameter_meta.clone(), parameter_meta],
            Ok(()),
        );
    }

    #[test]
    fn test_bpf_loader_serialize_aligned() {
        let loader_id = bpf_loader::id();
        let program_id = Pubkey::new_unique();
        let program_account =
            load_program_account_from_elf(&loader_id, "test_elfs/out/noop_aligned.so");
        let parameter_id = Pubkey::new_unique();
        let parameter_account = AccountSharedData::new(1, 0, &loader_id);
        let parameter_meta = AccountMeta {
            pubkey: parameter_id,
            is_signer: false,
            is_writable: false,
        };

        // Case: With program and parameter account
        process_instruction(
            &loader_id,
            &[0],
            &[],
            vec![
                (program_id, program_account.clone()),
                (parameter_id, parameter_account.clone()),
            ],
            vec![parameter_meta.clone()],
            Ok(()),
        );

        // Case: With duplicate accounts
        process_instruction(
            &loader_id,
            &[0],
            &[],
            vec![
                (program_id, program_account),
                (parameter_id, parameter_account),
            ],
            vec![parameter_meta.clone(), parameter_meta],
            Ok(()),
        );
    }

    #[test]
    fn test_bpf_loader_upgradeable_initialize_buffer() {
        let loader_id = bpf_loader_upgradeable::id();
        let buffer_address = Pubkey::new_unique();
        let buffer_account =
            AccountSharedData::new(1, UpgradeableLoaderState::size_of_buffer(9), &loader_id);
        let authority_address = Pubkey::new_unique();
        let authority_account =
            AccountSharedData::new(1, UpgradeableLoaderState::size_of_buffer(9), &loader_id);
        let instruction_data =
            bincode::serialize(&UpgradeableLoaderInstruction::InitializeBuffer).unwrap();
        let instruction_accounts = vec![
            AccountMeta {
                pubkey: buffer_address,
                is_signer: false,
                is_writable: true,
            },
            AccountMeta {
                pubkey: authority_address,
                is_signer: false,
                is_writable: false,
            },
        ];

        // Case: Success
        let accounts = process_instruction(
            &loader_id,
            &[],
            &instruction_data,
            vec![
                (buffer_address, buffer_account),
                (authority_address, authority_account),
            ],
            instruction_accounts.clone(),
            Ok(()),
        );
        let state: UpgradeableLoaderState = accounts.first().unwrap().state().unwrap();
        assert_eq!(
            state,
            UpgradeableLoaderState::Buffer {
                authority_address: Some(authority_address)
            }
        );

        // Case: Already initialized
        let accounts = process_instruction(
            &loader_id,
            &[],
            &instruction_data,
            vec![
                (buffer_address, accounts.first().unwrap().clone()),
                (authority_address, accounts.get(1).unwrap().clone()),
            ],
            instruction_accounts,
            Err(InstructionError::AccountAlreadyInitialized),
        );
        let state: UpgradeableLoaderState = accounts.first().unwrap().state().unwrap();
        assert_eq!(
            state,
            UpgradeableLoaderState::Buffer {
                authority_address: Some(authority_address)
            }
        );
    }

    #[test]
    fn test_bpf_loader_upgradeable_write() {
        let loader_id = bpf_loader_upgradeable::id();
        let buffer_address = Pubkey::new_unique();
        let mut buffer_account =
            AccountSharedData::new(1, UpgradeableLoaderState::size_of_buffer(9), &loader_id);
        let instruction_accounts = vec![
            AccountMeta {
                pubkey: buffer_address,
                is_signer: false,
                is_writable: true,
            },
            AccountMeta {
                pubkey: buffer_address,
                is_signer: true,
                is_writable: false,
            },
        ];

        // Case: Not initialized
        let instruction = bincode::serialize(&UpgradeableLoaderInstruction::Write {
            offset: 0,
            bytes: vec![42; 9],
        })
        .unwrap();
        process_instruction(
            &loader_id,
            &[],
            &instruction,
            vec![(buffer_address, buffer_account.clone())],
            instruction_accounts.clone(),
            Err(InstructionError::InvalidAccountData),
        );

        // Case: Write entire buffer
        let instruction = bincode::serialize(&UpgradeableLoaderInstruction::Write {
            offset: 0,
            bytes: vec![42; 9],
        })
        .unwrap();
        buffer_account
            .set_state(&UpgradeableLoaderState::Buffer {
                authority_address: Some(buffer_address),
            })
            .unwrap();
        let accounts = process_instruction(
            &loader_id,
            &[],
            &instruction,
            vec![(buffer_address, buffer_account.clone())],
            instruction_accounts.clone(),
            Ok(()),
        );
        let state: UpgradeableLoaderState = accounts.first().unwrap().state().unwrap();
        assert_eq!(
            state,
            UpgradeableLoaderState::Buffer {
                authority_address: Some(buffer_address)
            }
        );
        assert_eq!(
            &accounts
                .first()
                .unwrap()
                .data()
                .get(UpgradeableLoaderState::size_of_buffer_metadata()..)
                .unwrap(),
            &[42; 9]
        );

        // Case: Write portion of the buffer
        let instruction = bincode::serialize(&UpgradeableLoaderInstruction::Write {
            offset: 3,
            bytes: vec![42; 6],
        })
        .unwrap();
        let mut buffer_account =
            AccountSharedData::new(1, UpgradeableLoaderState::size_of_buffer(9), &loader_id);
        buffer_account
            .set_state(&UpgradeableLoaderState::Buffer {
                authority_address: Some(buffer_address),
            })
            .unwrap();
        let accounts = process_instruction(
            &loader_id,
            &[],
            &instruction,
            vec![(buffer_address, buffer_account.clone())],
            instruction_accounts.clone(),
            Ok(()),
        );
        let state: UpgradeableLoaderState = accounts.first().unwrap().state().unwrap();
        assert_eq!(
            state,
            UpgradeableLoaderState::Buffer {
                authority_address: Some(buffer_address)
            }
        );
        assert_eq!(
            &accounts
                .first()
                .unwrap()
                .data()
                .get(UpgradeableLoaderState::size_of_buffer_metadata()..)
                .unwrap(),
            &[0, 0, 0, 42, 42, 42, 42, 42, 42]
        );

        // Case: overflow size
        let instruction = bincode::serialize(&UpgradeableLoaderInstruction::Write {
            offset: 0,
            bytes: vec![42; 10],
        })
        .unwrap();
        buffer_account
            .set_state(&UpgradeableLoaderState::Buffer {
                authority_address: Some(buffer_address),
            })
            .unwrap();
        process_instruction(
            &loader_id,
            &[],
            &instruction,
            vec![(buffer_address, buffer_account.clone())],
            instruction_accounts.clone(),
            Err(InstructionError::AccountDataTooSmall),
        );

        // Case: overflow offset
        let instruction = bincode::serialize(&UpgradeableLoaderInstruction::Write {
            offset: 1,
            bytes: vec![42; 9],
        })
        .unwrap();
        buffer_account
            .set_state(&UpgradeableLoaderState::Buffer {
                authority_address: Some(buffer_address),
            })
            .unwrap();
        process_instruction(
            &loader_id,
            &[],
            &instruction,
            vec![(buffer_address, buffer_account.clone())],
            instruction_accounts.clone(),
            Err(InstructionError::AccountDataTooSmall),
        );

        // Case: Not signed
        let instruction = bincode::serialize(&UpgradeableLoaderInstruction::Write {
            offset: 0,
            bytes: vec![42; 9],
        })
        .unwrap();
        buffer_account
            .set_state(&UpgradeableLoaderState::Buffer {
                authority_address: Some(buffer_address),
            })
            .unwrap();
        process_instruction(
            &loader_id,
            &[],
            &instruction,
            vec![(buffer_address, buffer_account.clone())],
            vec![
                AccountMeta {
                    pubkey: buffer_address,
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: buffer_address,
                    is_signer: false,
                    is_writable: false,
                },
            ],
            Err(InstructionError::MissingRequiredSignature),
        );

        // Case: wrong authority
        let authority_address = Pubkey::new_unique();
        let instruction = bincode::serialize(&UpgradeableLoaderInstruction::Write {
            offset: 1,
            bytes: vec![42; 9],
        })
        .unwrap();
        buffer_account
            .set_state(&UpgradeableLoaderState::Buffer {
                authority_address: Some(buffer_address),
            })
            .unwrap();
        process_instruction(
            &loader_id,
            &[],
            &instruction,
            vec![
                (buffer_address, buffer_account.clone()),
                (authority_address, buffer_account.clone()),
            ],
            vec![
                AccountMeta {
                    pubkey: buffer_address,
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: authority_address,
                    is_signer: false,
                    is_writable: false,
                },
            ],
            Err(InstructionError::IncorrectAuthority),
        );

        // Case: None authority
        let instruction = bincode::serialize(&UpgradeableLoaderInstruction::Write {
            offset: 1,
            bytes: vec![42; 9],
        })
        .unwrap();
        buffer_account
            .set_state(&UpgradeableLoaderState::Buffer {
                authority_address: None,
            })
            .unwrap();
        process_instruction(
            &loader_id,
            &[],
            &instruction,
            vec![(buffer_address, buffer_account.clone())],
            instruction_accounts,
            Err(InstructionError::Immutable),
        );
    }

    fn truncate_data(account: &mut AccountSharedData, len: usize) {
        let mut data = account.data().to_vec();
        data.truncate(len);
        account.set_data(data);
    }

    #[test]
    fn test_bpf_loader_upgradeable_upgrade() {
        let mut file = File::open("test_elfs/out/noop_aligned.so").expect("file open failed");
        let mut elf_orig = Vec::new();
        file.read_to_end(&mut elf_orig).unwrap();
        let mut file = File::open("test_elfs/out/noop_unaligned.so").expect("file open failed");
        let mut elf_new = Vec::new();
        file.read_to_end(&mut elf_new).unwrap();
        assert_ne!(elf_orig.len(), elf_new.len());
        const SLOT: u64 = 42;
        let buffer_address = Pubkey::new_unique();
        let upgrade_authority_address = Pubkey::new_unique();

        fn get_accounts(
            buffer_address: &Pubkey,
            buffer_authority: &Pubkey,
            upgrade_authority_address: &Pubkey,
            elf_orig: &[u8],
            elf_new: &[u8],
        ) -> (Vec<(Pubkey, AccountSharedData)>, Vec<AccountMeta>) {
            let loader_id = bpf_loader_upgradeable::id();
            let program_address = Pubkey::new_unique();
            let spill_address = Pubkey::new_unique();
            let rent = Rent::default();
            let min_program_balance =
                1.max(rent.minimum_balance(UpgradeableLoaderState::size_of_program()));
            let min_programdata_balance = 1.max(rent.minimum_balance(
                UpgradeableLoaderState::size_of_programdata(elf_orig.len().max(elf_new.len())),
            ));
            let (programdata_address, _) =
                Pubkey::find_program_address(&[program_address.as_ref()], &loader_id);
            let mut buffer_account = AccountSharedData::new(
                1,
                UpgradeableLoaderState::size_of_buffer(elf_new.len()),
                &bpf_loader_upgradeable::id(),
            );
            buffer_account
                .set_state(&UpgradeableLoaderState::Buffer {
                    authority_address: Some(*buffer_authority),
                })
                .unwrap();
            buffer_account
                .data_as_mut_slice()
                .get_mut(UpgradeableLoaderState::size_of_buffer_metadata()..)
                .unwrap()
                .copy_from_slice(elf_new);
            let mut programdata_account = AccountSharedData::new(
                min_programdata_balance,
                UpgradeableLoaderState::size_of_programdata(elf_orig.len().max(elf_new.len())),
                &bpf_loader_upgradeable::id(),
            );
            programdata_account
                .set_state(&UpgradeableLoaderState::ProgramData {
                    slot: SLOT,
                    upgrade_authority_address: Some(*upgrade_authority_address),
                })
                .unwrap();
            let mut program_account = AccountSharedData::new(
                min_program_balance,
                UpgradeableLoaderState::size_of_program(),
                &bpf_loader_upgradeable::id(),
            );
            program_account.set_executable(true);
            program_account
                .set_state(&UpgradeableLoaderState::Program {
                    programdata_address,
                })
                .unwrap();
            let spill_account = AccountSharedData::new(0, 0, &Pubkey::new_unique());
            let rent_account = create_account_for_test(&rent);
            let clock_account = create_account_for_test(&Clock {
                slot: SLOT.saturating_add(1),
                ..Clock::default()
            });
            let upgrade_authority_account = AccountSharedData::new(1, 0, &Pubkey::new_unique());
            let transaction_accounts = vec![
                (programdata_address, programdata_account),
                (program_address, program_account),
                (*buffer_address, buffer_account),
                (spill_address, spill_account),
                (sysvar::rent::id(), rent_account),
                (sysvar::clock::id(), clock_account),
                (*upgrade_authority_address, upgrade_authority_account),
            ];
            let instruction_accounts = vec![
                AccountMeta {
                    pubkey: programdata_address,
                    is_signer: false,
                    is_writable: true,
                },
                AccountMeta {
                    pubkey: program_address,
                    is_signer: false,
                    is_writable: true,
                },
                AccountMeta {
                    pubkey: *buffer_address,
                    is_signer: false,
                    is_writable: true,
                },
                AccountMeta {
                    pubkey: spill_address,
                    is_signer: false,
                    is_writable: true,
                },
                AccountMeta {
                    pubkey: sysvar::rent::id(),
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: sysvar::clock::id(),
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: *upgrade_authority_address,
                    is_signer: true,
                    is_writable: false,
                },
            ];
            (transaction_accounts, instruction_accounts)
        }

        fn process_instruction(
            transaction_accounts: Vec<(Pubkey, AccountSharedData)>,
            instruction_accounts: Vec<AccountMeta>,
            expected_result: Result<(), InstructionError>,
        ) -> Vec<AccountSharedData> {
            let instruction_data =
                bincode::serialize(&UpgradeableLoaderInstruction::Upgrade).unwrap();
            mock_process_instruction(
                &bpf_loader_upgradeable::id(),
                Vec::new(),
                &instruction_data,
                transaction_accounts,
                instruction_accounts,
                None,
                None,
                expected_result,
                super::process_instruction,
            )
        }

        // Case: Success
        let (transaction_accounts, instruction_accounts) = get_accounts(
            &buffer_address,
            &upgrade_authority_address,
            &upgrade_authority_address,
            &elf_orig,
            &elf_new,
        );
        let accounts = process_instruction(transaction_accounts, instruction_accounts, Ok(()));
        let min_programdata_balance = Rent::default().minimum_balance(
            UpgradeableLoaderState::size_of_programdata(elf_orig.len().max(elf_new.len())),
        );
        assert_eq!(
            min_programdata_balance,
            accounts.first().unwrap().lamports()
        );
        assert_eq!(0, accounts.get(2).unwrap().lamports());
        assert_eq!(1, accounts.get(3).unwrap().lamports());
        let state: UpgradeableLoaderState = accounts.first().unwrap().state().unwrap();
        assert_eq!(
            state,
            UpgradeableLoaderState::ProgramData {
                slot: SLOT.saturating_add(1),
                upgrade_authority_address: Some(upgrade_authority_address)
            }
        );
        for (i, byte) in accounts
            .first()
            .unwrap()
            .data()
            .get(
                UpgradeableLoaderState::size_of_programdata_metadata()
                    ..UpgradeableLoaderState::size_of_programdata(elf_new.len()),
            )
            .unwrap()
            .iter()
            .enumerate()
        {
            assert_eq!(*elf_new.get(i).unwrap(), *byte);
        }

        // Case: not upgradable
        let (mut transaction_accounts, instruction_accounts) = get_accounts(
            &buffer_address,
            &upgrade_authority_address,
            &upgrade_authority_address,
            &elf_orig,
            &elf_new,
        );
        transaction_accounts
            .get_mut(0)
            .unwrap()
            .1
            .set_state(&UpgradeableLoaderState::ProgramData {
                slot: SLOT,
                upgrade_authority_address: None,
            })
            .unwrap();
        process_instruction(
            transaction_accounts,
            instruction_accounts,
            Err(InstructionError::Immutable),
        );

        // Case: wrong authority
        let (mut transaction_accounts, mut instruction_accounts) = get_accounts(
            &buffer_address,
            &upgrade_authority_address,
            &upgrade_authority_address,
            &elf_orig,
            &elf_new,
        );
        let invalid_upgrade_authority_address = Pubkey::new_unique();
        transaction_accounts.get_mut(6).unwrap().0 = invalid_upgrade_authority_address;
        instruction_accounts.get_mut(6).unwrap().pubkey = invalid_upgrade_authority_address;
        process_instruction(
            transaction_accounts,
            instruction_accounts,
            Err(InstructionError::IncorrectAuthority),
        );

        // Case: authority did not sign
        let (transaction_accounts, mut instruction_accounts) = get_accounts(
            &buffer_address,
            &upgrade_authority_address,
            &upgrade_authority_address,
            &elf_orig,
            &elf_new,
        );
        instruction_accounts.get_mut(6).unwrap().is_signer = false;
        process_instruction(
            transaction_accounts,
            instruction_accounts,
            Err(InstructionError::MissingRequiredSignature),
        );

        // Case: Buffer account and spill account alias
        let (transaction_accounts, mut instruction_accounts) = get_accounts(
            &buffer_address,
            &upgrade_authority_address,
            &upgrade_authority_address,
            &elf_orig,
            &elf_new,
        );
        *instruction_accounts.get_mut(3).unwrap() = instruction_accounts.get(2).unwrap().clone();
        process_instruction(
            transaction_accounts,
            instruction_accounts,
            Err(InstructionError::AccountBorrowFailed),
        );

        // Case: Programdata account and spill account alias
        let (transaction_accounts, mut instruction_accounts) = get_accounts(
            &buffer_address,
            &upgrade_authority_address,
            &upgrade_authority_address,
            &elf_orig,
            &elf_new,
        );
        *instruction_accounts.get_mut(3).unwrap() = instruction_accounts.get(0).unwrap().clone();
        process_instruction(
            transaction_accounts,
            instruction_accounts,
            Err(InstructionError::AccountBorrowFailed),
        );

        // Case: Program account not executable
        let (mut transaction_accounts, instruction_accounts) = get_accounts(
            &buffer_address,
            &upgrade_authority_address,
            &upgrade_authority_address,
            &elf_orig,
            &elf_new,
        );
        transaction_accounts
            .get_mut(1)
            .unwrap()
            .1
            .set_executable(false);
        process_instruction(
            transaction_accounts,
            instruction_accounts,
            Err(InstructionError::AccountNotExecutable),
        );

        // Case: Program account now owned by loader
        let (mut transaction_accounts, instruction_accounts) = get_accounts(
            &buffer_address,
            &upgrade_authority_address,
            &upgrade_authority_address,
            &elf_orig,
            &elf_new,
        );
        transaction_accounts
            .get_mut(1)
            .unwrap()
            .1
            .set_owner(Pubkey::new_unique());
        process_instruction(
            transaction_accounts,
            instruction_accounts,
            Err(InstructionError::IncorrectProgramId),
        );

        // Case: Program account not writable
        let (transaction_accounts, mut instruction_accounts) = get_accounts(
            &buffer_address,
            &upgrade_authority_address,
            &upgrade_authority_address,
            &elf_orig,
            &elf_new,
        );
        instruction_accounts.get_mut(1).unwrap().is_writable = false;
        process_instruction(
            transaction_accounts,
            instruction_accounts,
            Err(InstructionError::InvalidArgument),
        );

        // Case: Program account not initialized
        let (mut transaction_accounts, instruction_accounts) = get_accounts(
            &buffer_address,
            &upgrade_authority_address,
            &upgrade_authority_address,
            &elf_orig,
            &elf_new,
        );
        transaction_accounts
            .get_mut(1)
            .unwrap()
            .1
            .set_state(&UpgradeableLoaderState::Uninitialized)
            .unwrap();
        process_instruction(
            transaction_accounts,
            instruction_accounts,
            Err(InstructionError::InvalidAccountData),
        );

        // Case: Program ProgramData account mismatch
        let (mut transaction_accounts, mut instruction_accounts) = get_accounts(
            &buffer_address,
            &upgrade_authority_address,
            &upgrade_authority_address,
            &elf_orig,
            &elf_new,
        );
        let invalid_programdata_address = Pubkey::new_unique();
        transaction_accounts.get_mut(0).unwrap().0 = invalid_programdata_address;
        instruction_accounts.get_mut(0).unwrap().pubkey = invalid_programdata_address;
        process_instruction(
            transaction_accounts,
            instruction_accounts,
            Err(InstructionError::InvalidArgument),
        );

        // Case: Buffer account not initialized
        let (mut transaction_accounts, instruction_accounts) = get_accounts(
            &buffer_address,
            &upgrade_authority_address,
            &upgrade_authority_address,
            &elf_orig,
            &elf_new,
        );
        transaction_accounts
            .get_mut(2)
            .unwrap()
            .1
            .set_state(&UpgradeableLoaderState::Uninitialized)
            .unwrap();
        process_instruction(
            transaction_accounts,
            instruction_accounts,
            Err(InstructionError::InvalidArgument),
        );

        // Case: Buffer account too big
        let (mut transaction_accounts, instruction_accounts) = get_accounts(
            &buffer_address,
            &upgrade_authority_address,
            &upgrade_authority_address,
            &elf_orig,
            &elf_new,
        );
        transaction_accounts.get_mut(2).unwrap().1 = AccountSharedData::new(
            1,
            UpgradeableLoaderState::size_of_buffer(
                elf_orig.len().max(elf_new.len()).saturating_add(1),
            ),
            &bpf_loader_upgradeable::id(),
        );
        transaction_accounts
            .get_mut(2)
            .unwrap()
            .1
            .set_state(&UpgradeableLoaderState::Buffer {
                authority_address: Some(upgrade_authority_address),
            })
            .unwrap();
        process_instruction(
            transaction_accounts,
            instruction_accounts,
            Err(InstructionError::AccountDataTooSmall),
        );

        // Case: Buffer account too small
        let (mut transaction_accounts, instruction_accounts) = get_accounts(
            &buffer_address,
            &upgrade_authority_address,
            &upgrade_authority_address,
            &elf_orig,
            &elf_new,
        );
        transaction_accounts
            .get_mut(2)
            .unwrap()
            .1
            .set_state(&UpgradeableLoaderState::Buffer {
                authority_address: Some(upgrade_authority_address),
            })
            .unwrap();
        truncate_data(&mut transaction_accounts.get_mut(2).unwrap().1, 5);
        process_instruction(
            transaction_accounts,
            instruction_accounts,
            Err(InstructionError::InvalidAccountData),
        );

        // Case: Mismatched buffer and program authority
        let (transaction_accounts, instruction_accounts) = get_accounts(
            &buffer_address,
            &buffer_address,
            &upgrade_authority_address,
            &elf_orig,
            &elf_new,
        );
        process_instruction(
            transaction_accounts,
            instruction_accounts,
            Err(InstructionError::IncorrectAuthority),
        );

        // Case: No buffer authority
        let (mut transaction_accounts, instruction_accounts) = get_accounts(
            &buffer_address,
            &buffer_address,
            &upgrade_authority_address,
            &elf_orig,
            &elf_new,
        );
        transaction_accounts
            .get_mut(2)
            .unwrap()
            .1
            .set_state(&UpgradeableLoaderState::Buffer {
                authority_address: None,
            })
            .unwrap();
        process_instruction(
            transaction_accounts,
            instruction_accounts,
            Err(InstructionError::IncorrectAuthority),
        );

        // Case: No buffer and program authority
        let (mut transaction_accounts, instruction_accounts) = get_accounts(
            &buffer_address,
            &buffer_address,
            &upgrade_authority_address,
            &elf_orig,
            &elf_new,
        );
        transaction_accounts
            .get_mut(0)
            .unwrap()
            .1
            .set_state(&UpgradeableLoaderState::ProgramData {
                slot: SLOT,
                upgrade_authority_address: None,
            })
            .unwrap();
        transaction_accounts
            .get_mut(2)
            .unwrap()
            .1
            .set_state(&UpgradeableLoaderState::Buffer {
                authority_address: None,
            })
            .unwrap();
        process_instruction(
            transaction_accounts,
            instruction_accounts,
            Err(InstructionError::IncorrectAuthority),
        );
    }

    #[test]
    fn test_bpf_loader_upgradeable_set_upgrade_authority() {
        let instruction = bincode::serialize(&UpgradeableLoaderInstruction::SetAuthority).unwrap();
        let loader_id = bpf_loader_upgradeable::id();
        let slot = 0;
        let upgrade_authority_address = Pubkey::new_unique();
        let upgrade_authority_account = AccountSharedData::new(1, 0, &Pubkey::new_unique());
        let new_upgrade_authority_address = Pubkey::new_unique();
        let new_upgrade_authority_account = AccountSharedData::new(1, 0, &Pubkey::new_unique());
        let program_address = Pubkey::new_unique();
        let (programdata_address, _) = Pubkey::find_program_address(
            &[program_address.as_ref()],
            &bpf_loader_upgradeable::id(),
        );
        let mut programdata_account = AccountSharedData::new(
            1,
            UpgradeableLoaderState::size_of_programdata(0),
            &bpf_loader_upgradeable::id(),
        );
        programdata_account
            .set_state(&UpgradeableLoaderState::ProgramData {
                slot,
                upgrade_authority_address: Some(upgrade_authority_address),
            })
            .unwrap();
        let programdata_meta = AccountMeta {
            pubkey: programdata_address,
            is_signer: false,
            is_writable: true,
        };
        let upgrade_authority_meta = AccountMeta {
            pubkey: upgrade_authority_address,
            is_signer: true,
            is_writable: false,
        };
        let new_upgrade_authority_meta = AccountMeta {
            pubkey: new_upgrade_authority_address,
            is_signer: false,
            is_writable: false,
        };

        // Case: Set to new authority
        let accounts = process_instruction(
            &loader_id,
            &[],
            &instruction,
            vec![
                (programdata_address, programdata_account.clone()),
                (upgrade_authority_address, upgrade_authority_account.clone()),
                (
                    new_upgrade_authority_address,
                    new_upgrade_authority_account.clone(),
                ),
            ],
            vec![
                programdata_meta.clone(),
                upgrade_authority_meta.clone(),
                new_upgrade_authority_meta.clone(),
            ],
            Ok(()),
        );
        let state: UpgradeableLoaderState = accounts.first().unwrap().state().unwrap();
        assert_eq!(
            state,
            UpgradeableLoaderState::ProgramData {
                slot,
                upgrade_authority_address: Some(new_upgrade_authority_address),
            }
        );

        // Case: Not upgradeable
        let accounts = process_instruction(
            &loader_id,
            &[],
            &instruction,
            vec![
                (programdata_address, programdata_account.clone()),
                (upgrade_authority_address, upgrade_authority_account.clone()),
            ],
            vec![programdata_meta.clone(), upgrade_authority_meta.clone()],
            Ok(()),
        );
        let state: UpgradeableLoaderState = accounts.first().unwrap().state().unwrap();
        assert_eq!(
            state,
            UpgradeableLoaderState::ProgramData {
                slot,
                upgrade_authority_address: None,
            }
        );

        // Case: Authority did not sign
        process_instruction(
            &loader_id,
            &[],
            &instruction,
            vec![
                (programdata_address, programdata_account.clone()),
                (upgrade_authority_address, upgrade_authority_account.clone()),
            ],
            vec![
                programdata_meta.clone(),
                AccountMeta {
                    pubkey: upgrade_authority_address,
                    is_signer: false,
                    is_writable: false,
                },
            ],
            Err(InstructionError::MissingRequiredSignature),
        );

        // Case: wrong authority
        let invalid_upgrade_authority_address = Pubkey::new_unique();
        process_instruction(
            &loader_id,
            &[],
            &instruction,
            vec![
                (programdata_address, programdata_account.clone()),
                (
                    invalid_upgrade_authority_address,
                    upgrade_authority_account.clone(),
                ),
                (new_upgrade_authority_address, new_upgrade_authority_account),
            ],
            vec![
                programdata_meta.clone(),
                AccountMeta {
                    pubkey: invalid_upgrade_authority_address,
                    is_signer: true,
                    is_writable: false,
                },
                new_upgrade_authority_meta,
            ],
            Err(InstructionError::IncorrectAuthority),
        );

        // Case: No authority
        programdata_account
            .set_state(&UpgradeableLoaderState::ProgramData {
                slot,
                upgrade_authority_address: None,
            })
            .unwrap();
        process_instruction(
            &loader_id,
            &[],
            &instruction,
            vec![
                (programdata_address, programdata_account.clone()),
                (upgrade_authority_address, upgrade_authority_account.clone()),
            ],
            vec![programdata_meta.clone(), upgrade_authority_meta.clone()],
            Err(InstructionError::Immutable),
        );

        // Case: Not a ProgramData account
        programdata_account
            .set_state(&UpgradeableLoaderState::Program {
                programdata_address: Pubkey::new_unique(),
            })
            .unwrap();
        process_instruction(
            &loader_id,
            &[],
            &instruction,
            vec![
                (programdata_address, programdata_account.clone()),
                (upgrade_authority_address, upgrade_authority_account),
            ],
            vec![programdata_meta, upgrade_authority_meta],
            Err(InstructionError::InvalidArgument),
        );
    }

    #[test]
    fn test_bpf_loader_upgradeable_set_upgrade_authority_checked() {
        let instruction =
            bincode::serialize(&UpgradeableLoaderInstruction::SetAuthorityChecked).unwrap();
        let loader_id = bpf_loader_upgradeable::id();
        let slot = 0;
        let upgrade_authority_address = Pubkey::new_unique();
        let upgrade_authority_account = AccountSharedData::new(1, 0, &Pubkey::new_unique());
        let new_upgrade_authority_address = Pubkey::new_unique();
        let new_upgrade_authority_account = AccountSharedData::new(1, 0, &Pubkey::new_unique());
        let program_address = Pubkey::new_unique();
        let (programdata_address, _) = Pubkey::find_program_address(
            &[program_address.as_ref()],
            &bpf_loader_upgradeable::id(),
        );
        let mut programdata_account = AccountSharedData::new(
            1,
            UpgradeableLoaderState::size_of_programdata(0),
            &bpf_loader_upgradeable::id(),
        );
        programdata_account
            .set_state(&UpgradeableLoaderState::ProgramData {
                slot,
                upgrade_authority_address: Some(upgrade_authority_address),
            })
            .unwrap();
        let programdata_meta = AccountMeta {
            pubkey: programdata_address,
            is_signer: false,
            is_writable: true,
        };
        let upgrade_authority_meta = AccountMeta {
            pubkey: upgrade_authority_address,
            is_signer: true,
            is_writable: false,
        };
        let new_upgrade_authority_meta = AccountMeta {
            pubkey: new_upgrade_authority_address,
            is_signer: true,
            is_writable: false,
        };

        // Case: Set to new authority
        let accounts = process_instruction(
            &loader_id,
            &[],
            &instruction,
            vec![
                (programdata_address, programdata_account.clone()),
                (upgrade_authority_address, upgrade_authority_account.clone()),
                (
                    new_upgrade_authority_address,
                    new_upgrade_authority_account.clone(),
                ),
            ],
            vec![
                programdata_meta.clone(),
                upgrade_authority_meta.clone(),
                new_upgrade_authority_meta.clone(),
            ],
            Ok(()),
        );

        let state: UpgradeableLoaderState = accounts.first().unwrap().state().unwrap();
        assert_eq!(
            state,
            UpgradeableLoaderState::ProgramData {
                slot,
                upgrade_authority_address: Some(new_upgrade_authority_address),
            }
        );

        // Case: set to same authority
        process_instruction(
            &loader_id,
            &[],
            &instruction,
            vec![
                (programdata_address, programdata_account.clone()),
                (upgrade_authority_address, upgrade_authority_account.clone()),
            ],
            vec![
                programdata_meta.clone(),
                upgrade_authority_meta.clone(),
                upgrade_authority_meta.clone(),
            ],
            Ok(()),
        );

        // Case: present authority not in instruction
        process_instruction(
            &loader_id,
            &[],
            &instruction,
            vec![
                (programdata_address, programdata_account.clone()),
                (upgrade_authority_address, upgrade_authority_account.clone()),
                (
                    new_upgrade_authority_address,
                    new_upgrade_authority_account.clone(),
                ),
            ],
            vec![programdata_meta.clone(), new_upgrade_authority_meta.clone()],
            Err(InstructionError::NotEnoughAccountKeys),
        );

        // Case: new authority not in instruction
        process_instruction(
            &loader_id,
            &[],
            &instruction,
            vec![
                (programdata_address, programdata_account.clone()),
                (upgrade_authority_address, upgrade_authority_account.clone()),
                (
                    new_upgrade_authority_address,
                    new_upgrade_authority_account.clone(),
                ),
            ],
            vec![programdata_meta.clone(), upgrade_authority_meta.clone()],
            Err(InstructionError::NotEnoughAccountKeys),
        );

        // Case: present authority did not sign
        process_instruction(
            &loader_id,
            &[],
            &instruction,
            vec![
                (programdata_address, programdata_account.clone()),
                (upgrade_authority_address, upgrade_authority_account.clone()),
                (
                    new_upgrade_authority_address,
                    new_upgrade_authority_account.clone(),
                ),
            ],
            vec![
                programdata_meta.clone(),
                AccountMeta {
                    pubkey: upgrade_authority_address,
                    is_signer: false,
                    is_writable: false,
                },
                new_upgrade_authority_meta.clone(),
            ],
            Err(InstructionError::MissingRequiredSignature),
        );

        // Case: New authority did not sign
        process_instruction(
            &loader_id,
            &[],
            &instruction,
            vec![
                (programdata_address, programdata_account.clone()),
                (upgrade_authority_address, upgrade_authority_account.clone()),
                (
                    new_upgrade_authority_address,
                    new_upgrade_authority_account.clone(),
                ),
            ],
            vec![
                programdata_meta.clone(),
                upgrade_authority_meta.clone(),
                AccountMeta {
                    pubkey: new_upgrade_authority_address,
                    is_signer: false,
                    is_writable: false,
                },
            ],
            Err(InstructionError::MissingRequiredSignature),
        );

        // Case: wrong present authority
        let invalid_upgrade_authority_address = Pubkey::new_unique();
        process_instruction(
            &loader_id,
            &[],
            &instruction,
            vec![
                (programdata_address, programdata_account.clone()),
                (
                    invalid_upgrade_authority_address,
                    upgrade_authority_account.clone(),
                ),
                (new_upgrade_authority_address, new_upgrade_authority_account),
            ],
            vec![
                programdata_meta.clone(),
                AccountMeta {
                    pubkey: invalid_upgrade_authority_address,
                    is_signer: true,
                    is_writable: false,
                },
                new_upgrade_authority_meta.clone(),
            ],
            Err(InstructionError::IncorrectAuthority),
        );

        // Case: programdata is immutable
        programdata_account
            .set_state(&UpgradeableLoaderState::ProgramData {
                slot,
                upgrade_authority_address: None,
            })
            .unwrap();
        process_instruction(
            &loader_id,
            &[],
            &instruction,
            vec![
                (programdata_address, programdata_account.clone()),
                (upgrade_authority_address, upgrade_authority_account.clone()),
            ],
            vec![
                programdata_meta.clone(),
                upgrade_authority_meta.clone(),
                new_upgrade_authority_meta.clone(),
            ],
            Err(InstructionError::Immutable),
        );

        // Case: Not a ProgramData account
        programdata_account
            .set_state(&UpgradeableLoaderState::Program {
                programdata_address: Pubkey::new_unique(),
            })
            .unwrap();
        process_instruction(
            &loader_id,
            &[],
            &instruction,
            vec![
                (programdata_address, programdata_account.clone()),
                (upgrade_authority_address, upgrade_authority_account),
            ],
            vec![
                programdata_meta,
                upgrade_authority_meta,
                new_upgrade_authority_meta,
            ],
            Err(InstructionError::InvalidArgument),
        );
    }

    #[test]
    fn test_bpf_loader_upgradeable_set_buffer_authority() {
        let instruction = bincode::serialize(&UpgradeableLoaderInstruction::SetAuthority).unwrap();
        let loader_id = bpf_loader_upgradeable::id();
        let invalid_authority_address = Pubkey::new_unique();
        let authority_address = Pubkey::new_unique();
        let authority_account = AccountSharedData::new(1, 0, &Pubkey::new_unique());
        let new_authority_address = Pubkey::new_unique();
        let new_authority_account = AccountSharedData::new(1, 0, &Pubkey::new_unique());
        let buffer_address = Pubkey::new_unique();
        let mut buffer_account =
            AccountSharedData::new(1, UpgradeableLoaderState::size_of_buffer(0), &loader_id);
        buffer_account
            .set_state(&UpgradeableLoaderState::Buffer {
                authority_address: Some(authority_address),
            })
            .unwrap();
        let mut transaction_accounts = vec![
            (buffer_address, buffer_account.clone()),
            (authority_address, authority_account.clone()),
            (new_authority_address, new_authority_account.clone()),
        ];
        let buffer_meta = AccountMeta {
            pubkey: buffer_address,
            is_signer: false,
            is_writable: true,
        };
        let authority_meta = AccountMeta {
            pubkey: authority_address,
            is_signer: true,
            is_writable: false,
        };
        let new_authority_meta = AccountMeta {
            pubkey: new_authority_address,
            is_signer: false,
            is_writable: false,
        };

        // Case: New authority required
        let accounts = process_instruction(
            &loader_id,
            &[],
            &instruction,
            transaction_accounts.clone(),
            vec![buffer_meta.clone(), authority_meta.clone()],
            Err(InstructionError::IncorrectAuthority),
        );
        let state: UpgradeableLoaderState = accounts.first().unwrap().state().unwrap();
        assert_eq!(
            state,
            UpgradeableLoaderState::Buffer {
                authority_address: Some(authority_address),
            }
        );

        // Case: Set to new authority
        buffer_account
            .set_state(&UpgradeableLoaderState::Buffer {
                authority_address: Some(authority_address),
            })
            .unwrap();
        let accounts = process_instruction(
            &loader_id,
            &[],
            &instruction,
            transaction_accounts.clone(),
            vec![
                buffer_meta.clone(),
                authority_meta.clone(),
                new_authority_meta.clone(),
            ],
            Ok(()),
        );
        let state: UpgradeableLoaderState = accounts.first().unwrap().state().unwrap();
        assert_eq!(
            state,
            UpgradeableLoaderState::Buffer {
                authority_address: Some(new_authority_address),
            }
        );

        // Case: Authority did not sign
        process_instruction(
            &loader_id,
            &[],
            &instruction,
            transaction_accounts.clone(),
            vec![
                buffer_meta.clone(),
                AccountMeta {
                    pubkey: authority_address,
                    is_signer: false,
                    is_writable: false,
                },
                new_authority_meta.clone(),
            ],
            Err(InstructionError::MissingRequiredSignature),
        );

        // Case: wrong authority
        process_instruction(
            &loader_id,
            &[],
            &instruction,
            vec![
                (buffer_address, buffer_account.clone()),
                (invalid_authority_address, authority_account),
                (new_authority_address, new_authority_account),
            ],
            vec![
                buffer_meta.clone(),
                AccountMeta {
                    pubkey: invalid_authority_address,
                    is_signer: true,
                    is_writable: false,
                },
                new_authority_meta.clone(),
            ],
            Err(InstructionError::IncorrectAuthority),
        );

        // Case: No authority
        process_instruction(
            &loader_id,
            &[],
            &instruction,
            transaction_accounts.clone(),
            vec![buffer_meta.clone(), authority_meta.clone()],
            Err(InstructionError::IncorrectAuthority),
        );

        // Case: Set to no authority
        transaction_accounts
            .get_mut(0)
            .unwrap()
            .1
            .set_state(&UpgradeableLoaderState::Buffer {
                authority_address: None,
            })
            .unwrap();
        process_instruction(
            &loader_id,
            &[],
            &instruction,
            transaction_accounts.clone(),
            vec![
                buffer_meta.clone(),
                authority_meta.clone(),
                new_authority_meta.clone(),
            ],
            Err(InstructionError::Immutable),
        );

        // Case: Not a Buffer account
        transaction_accounts
            .get_mut(0)
            .unwrap()
            .1
            .set_state(&UpgradeableLoaderState::Program {
                programdata_address: Pubkey::new_unique(),
            })
            .unwrap();
        process_instruction(
            &loader_id,
            &[],
            &instruction,
            transaction_accounts.clone(),
            vec![buffer_meta, authority_meta, new_authority_meta],
            Err(InstructionError::InvalidArgument),
        );
    }

    #[test]
    fn test_bpf_loader_upgradeable_set_buffer_authority_checked() {
        let instruction =
            bincode::serialize(&UpgradeableLoaderInstruction::SetAuthorityChecked).unwrap();
        let loader_id = bpf_loader_upgradeable::id();
        let invalid_authority_address = Pubkey::new_unique();
        let authority_address = Pubkey::new_unique();
        let authority_account = AccountSharedData::new(1, 0, &Pubkey::new_unique());
        let new_authority_address = Pubkey::new_unique();
        let new_authority_account = AccountSharedData::new(1, 0, &Pubkey::new_unique());
        let buffer_address = Pubkey::new_unique();
        let mut buffer_account =
            AccountSharedData::new(1, UpgradeableLoaderState::size_of_buffer(0), &loader_id);
        buffer_account
            .set_state(&UpgradeableLoaderState::Buffer {
                authority_address: Some(authority_address),
            })
            .unwrap();
        let mut transaction_accounts = vec![
            (buffer_address, buffer_account.clone()),
            (authority_address, authority_account.clone()),
            (new_authority_address, new_authority_account.clone()),
        ];
        let buffer_meta = AccountMeta {
            pubkey: buffer_address,
            is_signer: false,
            is_writable: true,
        };
        let authority_meta = AccountMeta {
            pubkey: authority_address,
            is_signer: true,
            is_writable: false,
        };
        let new_authority_meta = AccountMeta {
            pubkey: new_authority_address,
            is_signer: true,
            is_writable: false,
        };

        // Case: Set to new authority
        buffer_account
            .set_state(&UpgradeableLoaderState::Buffer {
                authority_address: Some(authority_address),
            })
            .unwrap();
        let accounts = process_instruction(
            &loader_id,
            &[],
            &instruction,
            transaction_accounts.clone(),
            vec![
                buffer_meta.clone(),
                authority_meta.clone(),
                new_authority_meta.clone(),
            ],
            Ok(()),
        );
        let state: UpgradeableLoaderState = accounts.first().unwrap().state().unwrap();
        assert_eq!(
            state,
            UpgradeableLoaderState::Buffer {
                authority_address: Some(new_authority_address),
            }
        );

        // Case: set to same authority
        process_instruction(
            &loader_id,
            &[],
            &instruction,
            transaction_accounts.clone(),
            vec![
                buffer_meta.clone(),
                authority_meta.clone(),
                authority_meta.clone(),
            ],
            Ok(()),
        );

        // Case: Missing current authority
        process_instruction(
            &loader_id,
            &[],
            &instruction,
            transaction_accounts.clone(),
            vec![buffer_meta.clone(), new_authority_meta.clone()],
            Err(InstructionError::NotEnoughAccountKeys),
        );

        // Case: Missing new authority
        process_instruction(
            &loader_id,
            &[],
            &instruction,
            transaction_accounts.clone(),
            vec![buffer_meta.clone(), authority_meta.clone()],
            Err(InstructionError::NotEnoughAccountKeys),
        );

        // Case: wrong present authority
        process_instruction(
            &loader_id,
            &[],
            &instruction,
            vec![
                (buffer_address, buffer_account.clone()),
                (invalid_authority_address, authority_account),
                (new_authority_address, new_authority_account),
            ],
            vec![
                buffer_meta.clone(),
                AccountMeta {
                    pubkey: invalid_authority_address,
                    is_signer: true,
                    is_writable: false,
                },
                new_authority_meta.clone(),
            ],
            Err(InstructionError::IncorrectAuthority),
        );

        // Case: present authority did not sign
        process_instruction(
            &loader_id,
            &[],
            &instruction,
            transaction_accounts.clone(),
            vec![
                buffer_meta.clone(),
                AccountMeta {
                    pubkey: authority_address,
                    is_signer: false,
                    is_writable: false,
                },
                new_authority_meta.clone(),
            ],
            Err(InstructionError::MissingRequiredSignature),
        );

        // Case: new authority did not sign
        process_instruction(
            &loader_id,
            &[],
            &instruction,
            transaction_accounts.clone(),
            vec![
                buffer_meta.clone(),
                authority_meta.clone(),
                AccountMeta {
                    pubkey: new_authority_address,
                    is_signer: false,
                    is_writable: false,
                },
            ],
            Err(InstructionError::MissingRequiredSignature),
        );

        // Case: Not a Buffer account
        transaction_accounts
            .get_mut(0)
            .unwrap()
            .1
            .set_state(&UpgradeableLoaderState::Program {
                programdata_address: Pubkey::new_unique(),
            })
            .unwrap();
        process_instruction(
            &loader_id,
            &[],
            &instruction,
            transaction_accounts.clone(),
            vec![
                buffer_meta.clone(),
                authority_meta.clone(),
                new_authority_meta.clone(),
            ],
            Err(InstructionError::InvalidArgument),
        );

        // Case: Buffer is immutable
        transaction_accounts
            .get_mut(0)
            .unwrap()
            .1
            .set_state(&UpgradeableLoaderState::Buffer {
                authority_address: None,
            })
            .unwrap();
        process_instruction(
            &loader_id,
            &[],
            &instruction,
            transaction_accounts.clone(),
            vec![buffer_meta, authority_meta, new_authority_meta],
            Err(InstructionError::Immutable),
        );
    }

    #[test]
    fn test_bpf_loader_upgradeable_close() {
        let instruction = bincode::serialize(&UpgradeableLoaderInstruction::Close).unwrap();
        let loader_id = bpf_loader_upgradeable::id();
        let invalid_authority_address = Pubkey::new_unique();
        let authority_address = Pubkey::new_unique();
        let authority_account = AccountSharedData::new(1, 0, &Pubkey::new_unique());
        let recipient_address = Pubkey::new_unique();
        let recipient_account = AccountSharedData::new(1, 0, &Pubkey::new_unique());
        let buffer_address = Pubkey::new_unique();
        let mut buffer_account =
            AccountSharedData::new(1, UpgradeableLoaderState::size_of_buffer(128), &loader_id);
        buffer_account
            .set_state(&UpgradeableLoaderState::Buffer {
                authority_address: Some(authority_address),
            })
            .unwrap();
        let uninitialized_address = Pubkey::new_unique();
        let mut uninitialized_account = AccountSharedData::new(
            1,
            UpgradeableLoaderState::size_of_programdata(0),
            &loader_id,
        );
        uninitialized_account
            .set_state(&UpgradeableLoaderState::Uninitialized)
            .unwrap();
        let programdata_address = Pubkey::new_unique();
        let mut programdata_account = AccountSharedData::new(
            1,
            UpgradeableLoaderState::size_of_programdata(128),
            &loader_id,
        );
        programdata_account
            .set_state(&UpgradeableLoaderState::ProgramData {
                slot: 0,
                upgrade_authority_address: Some(authority_address),
            })
            .unwrap();
        let program_address = Pubkey::new_unique();
        let mut program_account =
            AccountSharedData::new(1, UpgradeableLoaderState::size_of_program(), &loader_id);
        program_account.set_executable(true);
        program_account
            .set_state(&UpgradeableLoaderState::Program {
                programdata_address,
            })
            .unwrap();
        let clock_account = create_account_for_test(&Clock {
            slot: 1,
            ..Clock::default()
        });
        let transaction_accounts = vec![
            (buffer_address, buffer_account.clone()),
            (recipient_address, recipient_account.clone()),
            (authority_address, authority_account.clone()),
        ];
        let buffer_meta = AccountMeta {
            pubkey: buffer_address,
            is_signer: false,
            is_writable: true,
        };
        let recipient_meta = AccountMeta {
            pubkey: recipient_address,
            is_signer: false,
            is_writable: true,
        };
        let authority_meta = AccountMeta {
            pubkey: authority_address,
            is_signer: true,
            is_writable: false,
        };

        // Case: close a buffer account
        let accounts = process_instruction(
            &loader_id,
            &[],
            &instruction,
            transaction_accounts,
            vec![
                buffer_meta.clone(),
                recipient_meta.clone(),
                authority_meta.clone(),
            ],
            Ok(()),
        );
        assert_eq!(0, accounts.first().unwrap().lamports());
        assert_eq!(2, accounts.get(1).unwrap().lamports());
        let state: UpgradeableLoaderState = accounts.first().unwrap().state().unwrap();
        assert_eq!(state, UpgradeableLoaderState::Uninitialized);
        assert_eq!(
            UpgradeableLoaderState::size_of_uninitialized(),
            accounts.first().unwrap().data().len()
        );

        // Case: close with wrong authority
        process_instruction(
            &loader_id,
            &[],
            &instruction,
            vec![
                (buffer_address, buffer_account.clone()),
                (recipient_address, recipient_account.clone()),
                (invalid_authority_address, authority_account.clone()),
            ],
            vec![
                buffer_meta,
                recipient_meta.clone(),
                AccountMeta {
                    pubkey: invalid_authority_address,
                    is_signer: true,
                    is_writable: false,
                },
            ],
            Err(InstructionError::IncorrectAuthority),
        );

        // Case: close an uninitialized account
        let accounts = process_instruction(
            &loader_id,
            &[],
            &instruction,
            vec![
                (uninitialized_address, uninitialized_account.clone()),
                (recipient_address, recipient_account.clone()),
                (invalid_authority_address, authority_account.clone()),
            ],
            vec![
                AccountMeta {
                    pubkey: uninitialized_address,
                    is_signer: false,
                    is_writable: true,
                },
                recipient_meta.clone(),
                authority_meta.clone(),
            ],
            Ok(()),
        );
        assert_eq!(0, accounts.first().unwrap().lamports());
        assert_eq!(2, accounts.get(1).unwrap().lamports());
        let state: UpgradeableLoaderState = accounts.first().unwrap().state().unwrap();
        assert_eq!(state, UpgradeableLoaderState::Uninitialized);
        assert_eq!(
            UpgradeableLoaderState::size_of_uninitialized(),
            accounts.first().unwrap().data().len()
        );

        // Case: close a program account
        let accounts = process_instruction(
            &loader_id,
            &[],
            &instruction,
            vec![
                (programdata_address, programdata_account.clone()),
                (recipient_address, recipient_account.clone()),
                (authority_address, authority_account.clone()),
                (program_address, program_account.clone()),
                (sysvar::clock::id(), clock_account.clone()),
            ],
            vec![
                AccountMeta {
                    pubkey: programdata_address,
                    is_signer: false,
                    is_writable: true,
                },
                recipient_meta,
                authority_meta,
                AccountMeta {
                    pubkey: program_address,
                    is_signer: false,
                    is_writable: true,
                },
            ],
            Ok(()),
        );
        assert_eq!(0, accounts.first().unwrap().lamports());
        assert_eq!(2, accounts.get(1).unwrap().lamports());
        let state: UpgradeableLoaderState = accounts.first().unwrap().state().unwrap();
        assert_eq!(state, UpgradeableLoaderState::Uninitialized);
        assert_eq!(
            UpgradeableLoaderState::size_of_uninitialized(),
            accounts.first().unwrap().data().len()
        );

        // Try to invoke closed account
        programdata_account = accounts.first().unwrap().clone();
        program_account = accounts.get(3).unwrap().clone();
        process_instruction(
            &program_address,
            &[0, 1],
            &[],
            vec![
                (programdata_address, programdata_account.clone()),
                (program_address, program_account.clone()),
            ],
            Vec::new(),
            Err(InstructionError::InvalidAccountData),
        );

        // Case: Reopen should fail
        process_instruction(
            &loader_id,
            &[],
            &bincode::serialize(&UpgradeableLoaderInstruction::DeployWithMaxDataLen {
                max_data_len: 0,
            })
            .unwrap(),
            vec![
                (recipient_address, recipient_account),
                (programdata_address, programdata_account),
                (program_address, program_account),
                (buffer_address, buffer_account),
                (
                    sysvar::rent::id(),
                    create_account_for_test(&Rent::default()),
                ),
                (sysvar::clock::id(), clock_account),
                (
                    system_program::id(),
                    AccountSharedData::new(0, 0, &system_program::id()),
                ),
                (authority_address, authority_account),
            ],
            vec![
                AccountMeta {
                    pubkey: recipient_address,
                    is_signer: true,
                    is_writable: true,
                },
                AccountMeta {
                    pubkey: programdata_address,
                    is_signer: false,
                    is_writable: true,
                },
                AccountMeta {
                    pubkey: program_address,
                    is_signer: false,
                    is_writable: true,
                },
                AccountMeta {
                    pubkey: buffer_address,
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: sysvar::rent::id(),
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: sysvar::clock::id(),
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: system_program::id(),
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: authority_address,
                    is_signer: false,
                    is_writable: false,
                },
            ],
            Err(InstructionError::AccountAlreadyInitialized),
        );
    }

    /// fuzzing utility function
    fn fuzz<F>(
        bytes: &[u8],
        outer_iters: usize,
        inner_iters: usize,
        offset: Range<usize>,
        value: Range<u8>,
        work: F,
    ) where
        F: Fn(&mut [u8]),
    {
        let mut rng = rand::thread_rng();
        for _ in 0..outer_iters {
            let mut mangled_bytes = bytes.to_vec();
            for _ in 0..inner_iters {
                let offset = rng.gen_range(offset.start, offset.end);
                let value = rng.gen_range(value.start, value.end);
                *mangled_bytes.get_mut(offset).unwrap() = value;
                work(&mut mangled_bytes);
            }
        }
    }

    #[test]
    #[ignore]
    fn test_fuzz() {
        let loader_id = bpf_loader::id();
        let program_id = Pubkey::new_unique();

        // Create program account
        let mut file = File::open("test_elfs/out/noop_aligned.so").expect("file open failed");
        let mut elf = Vec::new();
        file.read_to_end(&mut elf).unwrap();

        // Mangle the whole file
        fuzz(
            &elf,
            1_000_000_000,
            100,
            0..elf.len(),
            0..255,
            |bytes: &mut [u8]| {
                let mut program_account = AccountSharedData::new(1, 0, &loader_id);
                program_account.set_data(bytes.to_vec());
                program_account.set_executable(true);
                process_instruction(
                    &loader_id,
                    &[],
                    &[],
                    vec![(program_id, program_account)],
                    Vec::new(),
                    Ok(()),
                );
            },
        );
    }

    #[test]
    fn test_calculate_heap_cost() {
        let heap_cost = 8_u64;

        // heap allocations are in 32K block, `heap_cost` of CU is consumed per additional 32k

        // when `enable_heap_size_round_up` not enabled:
        {
            // assert less than 32K heap should cost zero unit
            assert_eq!(0, calculate_heap_cost(31_u64 * 1024, heap_cost, false));

            // assert exact 32K heap should be cost zero unit
            assert_eq!(0, calculate_heap_cost(32_u64 * 1024, heap_cost, false));

            // assert slightly more than 32K heap is mistakenly cost zero unit
            assert_eq!(0, calculate_heap_cost(33_u64 * 1024, heap_cost, false));

            // assert exact 64K heap should cost 1 * heap_cost
            assert_eq!(
                heap_cost,
                calculate_heap_cost(64_u64 * 1024, heap_cost, false)
            );
        }

        // when `enable_heap_size_round_up` is enabled:
        {
            // assert less than 32K heap should cost zero unit
            assert_eq!(0, calculate_heap_cost(31_u64 * 1024, heap_cost, true));

            // assert exact 32K heap should be cost zero unit
            assert_eq!(0, calculate_heap_cost(32_u64 * 1024, heap_cost, true));

            // assert slightly more than 32K heap should cost 1 * heap_cost
            assert_eq!(
                heap_cost,
                calculate_heap_cost(33_u64 * 1024, heap_cost, true)
            );

            // assert exact 64K heap should cost 1 * heap_cost
            assert_eq!(
                heap_cost,
                calculate_heap_cost(64_u64 * 1024, heap_cost, true)
            );
        }
    }
}
