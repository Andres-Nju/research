File_Code/solana/35c87c3888/bank/bank_after.rs --- Rust
19336             let account_size = rng.gen_range(1, MAX_PERMITTED_DATA_LENGTH) as usize;                                                                   19336             let account_size = rng.gen_range(
                                                                                                                                                             19337                 1,
                                                                                                                                                             19338                 MAX_PERMITTED_DATA_LENGTH as usize - MAX_PERMITTED_DATA_INCREASE,
                                                                                                                                                             19339             );

