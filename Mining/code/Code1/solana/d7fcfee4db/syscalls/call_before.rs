    fn call(
        &mut self,
        index: u64,
        meta_addr: u64,
        program_id_addr: u64,
        data_addr: u64,
        accounts_addr: u64,
        memory_mapping: &MemoryMapping,
        result: &mut Result<u64, EbpfError<BpfError>>,
    ) {
        let invoke_context = question_mark!(
            self.invoke_context
                .try_borrow()
                .map_err(|_| SyscallError::InvokeContextBorrowFailed),
            result
        );
        let loader_id = question_mark!(
            invoke_context
                .transaction_context
                .get_loader_key()
                .map_err(SyscallError::InstructionError),
            result
        );

        let budget = invoke_context.get_compute_budget();
        question_mark!(
            invoke_context
                .get_compute_meter()
                .consume(budget.syscall_base_cost),
            result
        );

        let stack_height = invoke_context.get_stack_height();
        let instruction_trace = invoke_context.get_instruction_trace();
        let instruction_context = if stack_height == TRANSACTION_LEVEL_STACK_HEIGHT {
            // pick one of the top-level instructions
            instruction_trace
                .len()
                .checked_sub(2)
                .and_then(|result| result.checked_sub(index as usize))
                .and_then(|index| instruction_trace.get(index))
                .and_then(|instruction_list| instruction_list.get(0))
        } else {
            // Walk the last list of inner instructions
            instruction_trace.last().and_then(|inners| {
                let mut current_index = 0;
                inners.iter().rev().skip(1).find(|(this_stack_height, _)| {
                    if stack_height == *this_stack_height {
                        if index == current_index {
                            return true;
                        } else {
                            current_index += 1;
                        }
                    }
                    false
                })
            })
        }
        .map(|(_, instruction_context)| instruction_context);

        if let Some(instruction_context) = instruction_context {
            let ProcessedSiblingInstruction {
                data_len,
                accounts_len,
            } = question_mark!(
                translate_type_mut::<ProcessedSiblingInstruction>(
                    memory_mapping,
                    meta_addr,
                    &loader_id
                ),
                result
            );

            if *data_len >= instruction_context.get_instruction_data().len()
                && *accounts_len == instruction_context.get_number_of_instruction_accounts()
            {
                let program_id = question_mark!(
                    translate_type_mut::<Pubkey>(memory_mapping, program_id_addr, &loader_id),
                    result
                );
                let data = question_mark!(
                    translate_slice_mut::<u8>(
                        memory_mapping,
                        data_addr,
                        *data_len as u64,
                        &loader_id,
                    ),
                    result
                );
                let accounts = question_mark!(
                    translate_slice_mut::<AccountMeta>(
                        memory_mapping,
                        accounts_addr,
                        *accounts_len as u64,
                        &loader_id,
                    ),
                    result
                );

                *program_id =
                    instruction_context.get_program_id(invoke_context.transaction_context);
                data.clone_from_slice(instruction_context.get_instruction_data());
                let account_metas = question_mark!(
                    (instruction_context.get_number_of_program_accounts()
                        ..instruction_context.get_number_of_accounts())
                        .map(|index_in_instruction| Ok(AccountMeta {
                            pubkey: *invoke_context.get_key_of_account_at_index(
                                instruction_context
                                    .get_index_in_transaction(index_in_instruction)?
                            )?,
                            is_signer: instruction_context.is_signer(index_in_instruction)?,
                            is_writable: instruction_context.is_writable(index_in_instruction)?,
                        }))
                        .collect::<Result<Vec<_>, InstructionError>>()
                        .map_err(SyscallError::InstructionError),
                    result
                );
                accounts.clone_from_slice(account_metas.as_slice());
            }
            *data_len = instruction_context.get_instruction_data().len();
            *accounts_len = instruction_context.get_number_of_instruction_accounts();
            *result = Ok(true as u64);
            return;
        }
        *result = Ok(false as u64);
    }
