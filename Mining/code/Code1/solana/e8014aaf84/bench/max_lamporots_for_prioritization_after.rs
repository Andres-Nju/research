pub fn max_lamports_for_prioritization(use_randomized_compute_unit_price: bool) -> u64 {
    if use_randomized_compute_unit_price {
        const MICRO_LAMPORTS_PER_LAMPORT: u64 = 1_000_000;
        let micro_lamport_fee: u128 = (MAX_COMPUTE_UNIT_PRICE as u128)
            .saturating_mul(TRANSFER_TRANSACTION_COMPUTE_UNIT as u128);
        let fee = micro_lamport_fee
            .saturating_add(MICRO_LAMPORTS_PER_LAMPORT.saturating_sub(1) as u128)
            .saturating_div(MICRO_LAMPORTS_PER_LAMPORT as u128);
        u64::try_from(fee).unwrap_or(u64::MAX)
    } else {
        0u64
    }
}
