File_Code/solana/0c76b89e55/system_instruction_processor/system_instruction_processor_after.rs --- Rust
357         RefCell::new(sysvar::recent_blockhashes::create_account_with_data(                                                                               357         RefCell::new(sysvar::rent::create_account(1, &Rent::free()))
358             1,                                                                                                                                               
359             vec![(0u64, &Hash::default()); 32].into_iter(),                                                                                                  

