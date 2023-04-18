fn run_program(
    name: &str,
    program_id: &Pubkey,
    parameter_accounts: Vec<KeyedAccount>,
    instruction_data: &[u8],
) -> Result<u64, InstructionError> {
    let path = create_bpf_path(name);
    let mut file = File::open(path).unwrap();

    let mut data = vec![];
    file.read_to_end(&mut data).unwrap();
    let loader_id = bpf_loader::id();
    let parameter_bytes = serialize_parameters(
        &bpf_loader::id(),
        program_id,
        &parameter_accounts,
        &instruction_data,
    )
    .unwrap();
    let mut invoke_context = MockInvokeContext::new(parameter_accounts);
    let compute_meter = invoke_context.get_compute_meter();
    let mut instruction_meter = ThisInstructionMeter { compute_meter };

    let config = Config {
        max_call_depth: 20,
        stack_frame_size: 4096,
        enable_instruction_meter: true,
        enable_instruction_tracing: true,
    };
    let mut executable = Executable::from_elf(&data, None, config).unwrap();
    executable.set_syscall_registry(register_syscalls(&mut invoke_context).unwrap());
    executable.jit_compile().unwrap();

    let mut instruction_count = 0;
    let mut tracer = None;
    for i in 0..2 {
        let mut parameter_bytes = parameter_bytes.clone();
        {
            let mut vm = create_vm(
                &loader_id,
                executable.as_ref(),
                parameter_bytes.as_slice_mut(),
                &mut invoke_context,
            )
            .unwrap();
            let result = if i == 0 {
                vm.execute_program_interpreted(&mut instruction_meter)
            } else {
                vm.execute_program_jit(&mut instruction_meter)
            };
            assert_eq!(SUCCESS, result.unwrap());
            if i == 1 {
                assert_eq!(instruction_count, vm.get_total_instruction_count());
            }
            instruction_count = vm.get_total_instruction_count();
            if config.enable_instruction_tracing {
                if i == 1 {
                    if !Tracer::compare(tracer.as_ref().unwrap(), vm.get_tracer()) {
                        let mut tracer_display = String::new();
                        tracer
                            .as_ref()
                            .unwrap()
                            .write(&mut tracer_display, vm.get_program())
                            .unwrap();
                        println!("TRACE (interpreted): {}", tracer_display);
                        let mut tracer_display = String::new();
                        vm.get_tracer()
                            .write(&mut tracer_display, vm.get_program())
                            .unwrap();
                        println!("TRACE (jit): {}", tracer_display);
                        assert!(false);
                    } else if log_enabled!(Trace) {
                        let mut trace_buffer = String::new();
                        tracer
                            .as_ref()
                            .unwrap()
                            .write(&mut trace_buffer, vm.get_program())
                            .unwrap();
                        trace!("BPF Program Instruction Trace:\n{}", trace_buffer);
                    }
                }
                tracer = Some(vm.get_tracer().clone());
            }
        }
        let parameter_accounts = invoke_context.get_keyed_accounts().unwrap();
        deserialize_parameters(
            &bpf_loader::id(),
            parameter_accounts,
            parameter_bytes.as_slice(),
            true,
        )
        .unwrap();
    }

    Ok(instruction_count)
}
