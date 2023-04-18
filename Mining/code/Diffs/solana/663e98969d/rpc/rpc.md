File_Code/solana/663e98969d/rpc/rpc_after.rs --- 1/3 --- Rust
1226         let tx = system_transaction::transfer(&alice, &alice.pubkey(), 20, blockhash);                                                                  1226         let tx = system_transaction::transfer(&alice, pubkey, std::u64::MAX, blockhash);

File_Code/solana/663e98969d/rpc/rpc_after.rs --- 2/3 --- Rust
1696         let tx = system_transaction::transfer(&alice, &alice.pubkey(), 20, blockhash);                                                                  1696         let tx = system_transaction::transfer(&alice, &bob_pubkey, std::u64::MAX, blockhash);

File_Code/solana/663e98969d/rpc/rpc_after.rs --- 3/3 --- Rust
1703             TransactionError::InstructionError(0, InstructionError::DuplicateAccountIndex),                                                             1703             TransactionError::InstructionError(0, InstructionError::CustomError(1)),

