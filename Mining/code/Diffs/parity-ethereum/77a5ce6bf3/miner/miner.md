File_Code/parity-ethereum/77a5ce6bf3/miner/miner_after.rs --- Rust
420                         MAX_SKIPPED_TRANSACTIONS.saturating_add((*open_block.block().header().gas_limit() / min_tx_gas).as_u64() as usize)               420                         MAX_SKIPPED_TRANSACTIONS.saturating_add(cmp::min(*open_block.block().header().gas_limit() / min_tx_gas, u64::max_value().into())
                                                                                                                                                                 .as_u64() as usize)

