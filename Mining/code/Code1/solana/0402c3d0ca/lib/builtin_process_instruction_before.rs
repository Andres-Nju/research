pub fn builtin_process_instruction(
    process_instruction: solana_sdk::entrypoint::ProcessInstruction,
    _first_instruction_account: usize,
    invoke_context: &mut InvokeContext,
) -> Result<(), InstructionError> {
    set_invoke_context(invoke_context);

    let transaction_context = &invoke_context.transaction_context;
    let instruction_context = transaction_context.get_current_instruction_context()?;
    let instruction_data = instruction_context.get_instruction_data();
    let instruction_account_indices = 0..instruction_context.get_number_of_instruction_accounts();

    let log_collector = invoke_context.get_log_collector();
    let program_id = instruction_context.get_last_program_key(transaction_context)?;
    stable_log::program_invoke(
        &log_collector,
        program_id,
        invoke_context.get_stack_height(),
    );

    // Copy indices_in_instruction into a HashSet to ensure there are no duplicates
    let deduplicated_indices: HashSet<usize> = instruction_account_indices.collect();

    // Serialize entrypoint parameters with BPF ABI
    let (mut parameter_bytes, _account_lengths) = serialize_parameters(
        invoke_context.transaction_context,
        invoke_context
            .transaction_context
            .get_current_instruction_context()?,
    )?;

    // Deserialize data back into instruction params
    let (program_id, account_infos, _input) =
        unsafe { deserialize(&mut parameter_bytes.as_slice_mut()[0] as *mut u8) };

    // Execute the program
    process_instruction(program_id, &account_infos, instruction_data).map_err(|err| {
        let err = u64::from(err);
        stable_log::program_failure(&log_collector, program_id, &err.into());
        err
    })?;
    stable_log::program_success(&log_collector, program_id);

    // Lookup table for AccountInfo
    let account_info_map: HashMap<_, _> = account_infos.into_iter().map(|a| (a.key, a)).collect();

    // Re-fetch the instruction context. The previous reference may have been
    // invalidated due to the `set_invoke_context` in a CPI.
    let transaction_context = &invoke_context.transaction_context;
    let instruction_context = transaction_context.get_current_instruction_context()?;

    // Commit AccountInfo changes back into KeyedAccounts
    for i in deduplicated_indices.into_iter() {
        let mut borrowed_account =
            instruction_context.try_borrow_instruction_account(transaction_context, i)?;
        if borrowed_account.is_writable() {
            if let Some(account_info) = account_info_map.get(borrowed_account.get_key()) {
                if borrowed_account.get_lamports() != account_info.lamports() {
                    borrowed_account.set_lamports(account_info.lamports())?;
                }

                if borrowed_account
                    .can_data_be_resized(account_info.data_len())
                    .is_ok()
                    && borrowed_account.can_data_be_changed().is_ok()
                {
                    borrowed_account.set_data(&account_info.data.borrow())?;
                }
                if borrowed_account.get_owner() != account_info.owner {
                    borrowed_account.set_owner(account_info.owner.as_ref())?;
                }
            }
        }
    }

    Ok(())
}
