File_Code/solana/1dd467ed7d/bank/bank_after.rs --- 1/2 --- Rust
395         }                                                                                                                                                395         } else if tail.len() > WINDOW_SIZE as usize {
                                                                                                                                                             396             tail = tail[tail.len() - WINDOW_SIZE as usize..].to_vec();
                                                                                                                                                             397         }

File_Code/solana/1dd467ed7d/bank/bank_after.rs --- 2/2 --- Rust
776         for entry_count in window_size - 1..window_size + 1 {                                                                                            778         for entry_count in window_size - 1..window_size + 3 {

