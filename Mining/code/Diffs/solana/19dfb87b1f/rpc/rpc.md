File_Code/solana/19dfb87b1f/rpc/rpc_after.rs --- 1/2 --- Rust
274     let transaction_count = rpc_client                                                                                                                   274     let mut transaction_count = rpc_client

File_Code/solana/19dfb87b1f/rpc/rpc_after.rs --- 2/2 --- Rust
284     let mut x = 0;                                                                                                                                       ... 
285     let now = Instant::now();                                                                                                                            284     let now = Instant::now();
...                                                                                                                                                          285     let expected_transaction_count = transaction_count + transactions.len() as u64;
286     while x < transaction_count + 500 || now.elapsed() > Duration::from_secs(5) {                                                                        286     while transaction_count < expected_transaction_count && now.elapsed() < Duration::from_secs(5) {
287         x = rpc_client                                                                                                                                   287         transaction_count = rpc_client

