pub fn close_any(
    close_address: &Pubkey,
    recipient_address: &Pubkey,
    authority_address: Option<&Pubkey>,
    program_address: Option<&Pubkey>,
) -> Instruction {
    let mut metas = vec![
        AccountMeta::new(*close_address, false),
        AccountMeta::new(*recipient_address, false),
    ];
    if let Some(authority_address) = authority_address {
        metas.push(AccountMeta::new(*authority_address, true));
    }
    if let Some(program_address) = program_address {
        metas.push(AccountMeta::new(*program_address, false));
    }
    Instruction::new_with_bincode(id(), &UpgradeableLoaderInstruction::Close, metas)
}
