fn create_executor_from_bytes(
    feature_set: &FeatureSet,
    compute_budget: &ComputeBudget,
    log_collector: Option<Rc<RefCell<LogCollector>>>,
    create_executor_metrics: &mut CreateMetrics,
    programdata: &[u8],
    use_jit: bool,
    reject_deployment_of_broken_elfs: bool,
) -> Result<Arc<BpfExecutor>, InstructionError> {
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
    create_executor_metrics.register_syscalls_us = register_syscalls_time.as_us();
    let mut load_elf_time = Measure::start("load_elf_time");
    let executable = Executable::<InvokeContext>::from_elf(programdata, loader).map_err(|err| {
        ic_logger_msg!(log_collector, "{}", err);
        InstructionError::InvalidAccountData
    });
    load_elf_time.stop();
    create_executor_metrics.load_elf_us = load_elf_time.as_us();
    let executable = executable?;
    let mut verify_code_time = Measure::start("verify_code_time");
    let mut verified_executable =
        VerifiedExecutable::<RequisiteVerifier, InvokeContext>::from_executable(executable)
            .map_err(|err| {
                ic_logger_msg!(log_collector, "{}", err);
                InstructionError::InvalidAccountData
            })?;
    verify_code_time.stop();
    create_executor_metrics.verify_code_us = verify_code_time.as_us();
    #[cfg(all(not(target_os = "windows"), target_arch = "x86_64"))]
    if use_jit {
        let mut jit_compile_time = Measure::start("jit_compile_time");
        let jit_compile_result = verified_executable.jit_compile();
        jit_compile_time.stop();
        create_executor_metrics.jit_compile_us = jit_compile_time.as_us();
        if let Err(err) = jit_compile_result {
            ic_logger_msg!(log_collector, "Failed to compile program {:?}", err);
            return Err(InstructionError::ProgramFailedToCompile);
        }
    }
    Ok(Arc::new(BpfExecutor {
        verified_executable,
        use_jit,
    }))
}
