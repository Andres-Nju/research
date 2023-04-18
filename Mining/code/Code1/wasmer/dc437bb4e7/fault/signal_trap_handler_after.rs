extern "C" fn signal_trap_handler(
    signum: ::nix::libc::c_int,
    siginfo: *mut siginfo_t,
    ucontext: *mut c_void,
) {
    unsafe {
        let fault = get_fault_info(siginfo as _, ucontext);

        let mut unwind_result: Box<dyn Any> = Box::new(());

        let should_unwind = allocate_and_run(TRAP_STACK_SIZE, || {
            let mut is_suspend_signal = false;

            WAS_SIGINT_TRIGGERED.with(|x| x.set(false));

            match Signal::from_c_int(signum) {
                Ok(SIGTRAP) => {
                    // breakpoint
                    let out: Option<Result<(), Box<dyn Any>>> = with_breakpoint_map(|bkpt_map| {
                        bkpt_map.and_then(|x| x.get(&(fault.ip as usize))).map(|x| {
                            x(BreakpointInfo {
                                fault: Some(&fault),
                            })
                        })
                    });
                    match out {
                        Some(Ok(())) => {
                            return false;
                        }
                        Some(Err(e)) => {
                            unwind_result = e;
                            return true;
                        }
                        None => {}
                    }
                }
                Ok(SIGSEGV) | Ok(SIGBUS) => {
                    if fault.faulting_addr as usize == get_wasm_interrupt_signal_mem() as usize {
                        is_suspend_signal = true;
                        clear_wasm_interrupt();
                        if INTERRUPT_SIGNAL_DELIVERED.swap(false, Ordering::SeqCst) {
                            WAS_SIGINT_TRIGGERED.with(|x| x.set(true));
                        }
                    }
                }
                _ => {}
            }

            let ctx: &mut vm::Ctx = &mut **CURRENT_CTX.with(|x| x.get());
            let rsp = fault.known_registers[X64Register::GPR(GPR::RSP).to_index().0].unwrap();

            let es_image = CURRENT_CODE_VERSIONS.with(|versions| {
                let versions = versions.borrow();
                read_stack(
                    || versions.iter(),
                    rsp as usize as *const u64,
                    fault.known_registers,
                    Some(fault.ip as usize as u64),
                )
            });

            if is_suspend_signal {
                let image = build_instance_image(ctx, es_image);
                unwind_result = Box::new(image);
            } else {
                if es_image.frames.len() > 0 {
                    eprintln!(
                        "\n{}",
                        "Wasmer encountered an error while running your WebAssembly program."
                    );
                    es_image.print_backtrace_if_needed();
                }
                // Just let the error propagate otherwise
            }

            true
        });

        if should_unwind {
            begin_unsafe_unwind(unwind_result);
        }
    }
}
