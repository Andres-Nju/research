File_Code/solana/d7ba15cde8/block_cost_limits/block_cost_limits_after.rs --- 1/2 --- Rust
18     MAX_INSTRUCTION_COST * max_instructions_per_block()                                                                                                   18     MAX_INSTRUCTION_COST * max_instructions_per_block() * 10

File_Code/solana/d7ba15cde8/block_cost_limits/block_cost_limits_after.rs --- 2/2 --- Rust
26     block_cost_max() / MAX_BLOCK_TIME_US                                                                                                                  26     (MAX_INSTRUCTION_COST / AVG_INSTRUCTION_TIME_US) * SYSTEM_PARALLELISM

