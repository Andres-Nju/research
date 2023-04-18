pub fn register_syscalls(
    invoke_context: &mut dyn InvokeContext,
) -> Result<SyscallRegistry, EbpfError<BpfError>> {
    let mut syscall_registry = SyscallRegistry::default();

    syscall_registry.register_syscall_by_name(b"abort", SyscallAbort::call)?;
    syscall_registry.register_syscall_by_name(b"sol_panic_", SyscallPanic::call)?;
    syscall_registry.register_syscall_by_name(b"sol_log_", SyscallLog::call)?;
    syscall_registry.register_syscall_by_name(b"sol_log_64_", SyscallLogU64::call)?;

    syscall_registry
        .register_syscall_by_name(b"sol_log_compute_units_", SyscallLogBpfComputeUnits::call)?;

    syscall_registry.register_syscall_by_name(b"sol_log_pubkey", SyscallLogPubkey::call)?;

    syscall_registry.register_syscall_by_name(
        b"sol_create_program_address",
        SyscallCreateProgramAddress::call,
    )?;
    syscall_registry.register_syscall_by_name(
        b"sol_try_find_program_address",
        SyscallTryFindProgramAddress::call,
    )?;

    syscall_registry.register_syscall_by_name(b"sol_sha256", SyscallSha256::call)?;
    syscall_registry.register_syscall_by_name(b"sol_keccak256", SyscallKeccak256::call)?;

    if invoke_context.is_feature_active(&secp256k1_recover_syscall_enabled::id()) {
        syscall_registry
            .register_syscall_by_name(b"sol_secp256k1_recover", SyscallSecp256k1Recover::call)?;
    }

    if invoke_context.is_feature_active(&blake3_syscall_enabled::id()) {
        syscall_registry.register_syscall_by_name(b"sol_blake3", SyscallBlake3::call)?;
    }

    syscall_registry
        .register_syscall_by_name(b"sol_get_clock_sysvar", SyscallGetClockSysvar::call)?;
    syscall_registry.register_syscall_by_name(
        b"sol_get_epoch_schedule_sysvar",
        SyscallGetEpochScheduleSysvar::call,
    )?;
    if invoke_context.is_feature_active(&disable_fees_sysvar::id()) {
        syscall_registry
            .register_syscall_by_name(b"sol_get_fees_sysvar", SyscallGetFeesSysvar::call)?;
    }
    syscall_registry
        .register_syscall_by_name(b"sol_get_rent_sysvar", SyscallGetRentSysvar::call)?;

    syscall_registry.register_syscall_by_name(b"sol_memcpy_", SyscallMemcpy::call)?;
    syscall_registry.register_syscall_by_name(b"sol_memmove_", SyscallMemmove::call)?;
    syscall_registry.register_syscall_by_name(b"sol_memcmp_", SyscallMemcmp::call)?;
    syscall_registry.register_syscall_by_name(b"sol_memset_", SyscallMemset::call)?;

    // Cross-program invocation syscalls
    syscall_registry
        .register_syscall_by_name(b"sol_invoke_signed_c", SyscallInvokeSignedC::call)?;
    syscall_registry
        .register_syscall_by_name(b"sol_invoke_signed_rust", SyscallInvokeSignedRust::call)?;

    // Memory allocator
    syscall_registry.register_syscall_by_name(b"sol_alloc_free_", SyscallAllocFree::call)?;

    // Return data
    if invoke_context.is_feature_active(&return_data_syscall_enabled::id()) {
        syscall_registry
            .register_syscall_by_name(b"sol_set_return_data", SyscallSetReturnData::call)?;
        syscall_registry
            .register_syscall_by_name(b"sol_get_return_data", SyscallGetReturnData::call)?;
    }

    // Log data
    if invoke_context.is_feature_active(&sol_log_data_syscall_enabled::id()) {
        syscall_registry.register_syscall_by_name(b"sol_log_data", SyscallLogData::call)?;
    }

    Ok(syscall_registry)
}
