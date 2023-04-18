File_Code/solana/611d2ec73c/validator/validator_after.rs --- Rust
704         block_commitment_cache.initialize_slots(                                                                                                           . 
705             bank_forks.read().unwrap().working_bank().slot(),                                                                                            704         let bank_forks_guard = bank_forks.read().unwrap();
...                                                                                                                                                          705         block_commitment_cache.initialize_slots(
706             bank_forks.read().unwrap().root(),                                                                                                           706             bank_forks_guard.working_bank().slot(),
...                                                                                                                                                          707             bank_forks_guard.root(),
...                                                                                                                                                          708         );
707         );                                                                                                                                               709         drop(bank_forks_guard);

