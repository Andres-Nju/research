    fn execute(
        &self,
        program_id: &Pubkey,
        keyed_accounts: &[KeyedAccount],
        instruction_data: &[u8],
        invoke_context: &mut dyn InvokeContext,
    ) -> Result<(), InstructionError> {
        let logger = invoke_context.get_logger();
        let invoke_depth = invoke_context.invoke_depth();

        let mut keyed_accounts_iter = keyed_accounts.iter();
        let program = next_keyed_account(&mut keyed_accounts_iter)?;

        let parameter_accounts = keyed_accounts_iter.as_slice();
        let parameter_bytes = serialize_parameters(
            program_id,
            program.unsigned_key(),
            parameter_accounts,
            &instruction_data,
        )?;
        {
            let compute_meter = invoke_context.get_compute_meter();
            let mut vm = match create_vm(
                program_id,
                self.executable.as_ref(),
                parameter_bytes.as_slice(),
                &parameter_accounts,
                invoke_context,
            ) {
                Ok(info) => info,
                Err(e) => {
                    log!(logger, "Failed to create BPF VM: {}", e);
                    return Err(BPFLoaderError::VirtualMachineCreationFailed.into());
                }
            };

            stable_log::program_invoke(&logger, program.unsigned_key(), invoke_depth);
            let mut instruction_meter = ThisInstructionMeter::new(compute_meter.clone());
            let before = compute_meter.borrow().get_remaining();
            const IS_JIT_ENABLED: bool = false;
            let result = if IS_JIT_ENABLED {
                if vm.jit_compile().is_err() {
                    return Err(BPFLoaderError::VirtualMachineCreationFailed.into());
                }
                unsafe { vm.execute_program_jit(&mut instruction_meter) }
            } else {
                vm.execute_program_interpreted(&mut instruction_meter)
            };
            let after = compute_meter.borrow().get_remaining();
            log!(
                logger,
                "Program {} consumed {} of {} compute units",
                program.unsigned_key(),
                before - after,
                before
            );
            match result {
                Ok(status) => {
                    if status != SUCCESS {
                        let error: InstructionError = status.into();
                        stable_log::program_failure(&logger, program.unsigned_key(), &error);
                        return Err(error);
                    }
                }
                Err(error) => {
                    let error = match error {
                        EbpfError::UserError(BPFError::SyscallError(
                            SyscallError::InstructionError(error),
                        )) => error,
                        _ => BPFLoaderError::VirtualMachineFailedToRunProgram.into(),
                    };

                    stable_log::program_failure(&logger, program.unsigned_key(), &error);
                    return Err(error);
                }
            }
        }
        deserialize_parameters(program_id, parameter_accounts, &parameter_bytes)?;
        stable_log::program_success(&logger, program.unsigned_key());
        Ok(())
    }
