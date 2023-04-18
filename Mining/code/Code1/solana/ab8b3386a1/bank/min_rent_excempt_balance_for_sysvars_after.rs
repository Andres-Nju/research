    fn min_rent_exempt_balance_for_sysvars(bank: &Bank, sysvar_ids: &[Pubkey]) -> u64 {
        sysvar_ids
            .iter()
            .map(|sysvar_id| {
                trace!("min_rent_excempt_balance_for_sysvars: {}", sysvar_id);
                bank.get_minimum_balance_for_rent_exemption(
                    bank.get_account(sysvar_id).unwrap().data().len(),
                )
            })
            .sum()
    }
