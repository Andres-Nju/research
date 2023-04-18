pub const fn block_cost_max() -> u64 {
    MAX_INSTRUCTION_COST * max_instructions_per_block()
}
