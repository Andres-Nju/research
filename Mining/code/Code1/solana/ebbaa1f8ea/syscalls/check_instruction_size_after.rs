fn check_instruction_size(
    num_accounts: usize,
    data_len: usize,
    invoke_context: &Ref<&mut dyn InvokeContext>,
) -> Result<(), EbpfError<BPFError>> {
    let size = num_accounts
        .saturating_mul(size_of::<AccountMeta>())
        .saturating_add(data_len);
    let max_size = invoke_context
        .get_bpf_compute_budget()
        .max_cpi_instruction_size;
    if size > max_size {
        return Err(SyscallError::InstructionTooLarge(size, max_size).into());
    }
    Ok(())
}
