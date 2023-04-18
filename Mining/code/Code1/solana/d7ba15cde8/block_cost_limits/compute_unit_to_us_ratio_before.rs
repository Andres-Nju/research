pub const fn compute_unit_to_us_ratio() -> u64 {
    block_cost_max() / MAX_BLOCK_TIME_US
}
