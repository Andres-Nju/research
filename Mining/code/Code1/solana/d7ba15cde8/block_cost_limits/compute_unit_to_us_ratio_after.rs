pub const fn compute_unit_to_us_ratio() -> u64 {
    (MAX_INSTRUCTION_COST / AVG_INSTRUCTION_TIME_US) * SYSTEM_PARALLELISM
}
