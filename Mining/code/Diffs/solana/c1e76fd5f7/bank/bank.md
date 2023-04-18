File_Code/solana/c1e76fd5f7/bank/bank_after.rs --- 1/2 --- Rust
174             count.fetch_add(1, Ordering::Relaxed);                                                                                                       174             count.fetch_add(1, Relaxed);

File_Code/solana/c1e76fd5f7/bank/bank_after.rs --- 2/2 --- Rust
185                     let count = count.load(Ordering::Relaxed);                                                                                           185                     let count = count.load(Relaxed);

