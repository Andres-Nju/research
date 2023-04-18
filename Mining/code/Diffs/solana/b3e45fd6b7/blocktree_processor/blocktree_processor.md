File_Code/solana/b3e45fd6b7/blocktree_processor/blocktree_processor_after.rs --- 1/2 --- Rust
39             for r in results {                                                                                                                            39             for (r, tx) in results.iter().zip(e.transactions.iter()) {

File_Code/solana/b3e45fd6b7/blocktree_processor/blocktree_processor_after.rs --- 2/2 --- Rust
45                         warn!("Unexpected validator error: {:?}", e);                                                                                     45                         warn!("Unexpected validator error: {:?}, tx: {:?}", e, tx);
46                         datapoint!(                                                                                                                       46                         datapoint!(
47                             "validator_process_entry_error",                                                                                              47                             "validator_process_entry_error",
48                             ("error", format!("{:?}", e), String)                                                                                         48                             ("error", format!("error: {:?}, tx: {:?}", e, tx), String)

