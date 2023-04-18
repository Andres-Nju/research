File_Code/solana/ab8b3386a1/bank/bank_after.rs --- 1/3 --- Rust
9058             min_rent_excempt_balance_for_sysvars(&bank, &[sysvar::slot_history::id()]);                                                                 9058             min_rent_exempt_balance_for_sysvars(&bank, &[sysvar::slot_history::id()]);

File_Code/solana/ab8b3386a1/bank/bank_after.rs --- 2/3 --- Rust
12073                     old + min_rent_excempt_balance_for_sysvars(&bank1, &[sysvar::clock::id()]),                                                        12073                     old + min_rent_exempt_balance_for_sysvars(&bank1, &[sysvar::clock::id()]),

File_Code/solana/ab8b3386a1/bank/bank_after.rs --- 3/3 --- Rust
15812     fn min_rent_excempt_balance_for_sysvars(bank: &Bank, sysvar_ids: &[Pubkey]) -> u64 {                                                               15812     fn min_rent_exempt_balance_for_sysvars(bank: &Bank, sysvar_ids: &[Pubkey]) -> u64 {

