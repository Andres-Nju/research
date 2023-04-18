File_Code/solana/d28536d76e/bank/bank_after.rs --- 1/3 --- Rust
45     DuplicateSiganture(Signature),                                                                                                                        45     DuplicateSignature(Signature),

File_Code/solana/d28536d76e/bank/bank_after.rs --- 2/3 --- Rust
138             return Err(BankError::DuplicateSiganture(*sig));                                                                                             138             return Err(BankError::DuplicateSignature(*sig));

File_Code/solana/d28536d76e/bank/bank_after.rs --- 3/3 --- Rust
603             Err(BankError::DuplicateSiganture(sig))                                                                                                      603             Err(BankError::DuplicateSignature(sig))

