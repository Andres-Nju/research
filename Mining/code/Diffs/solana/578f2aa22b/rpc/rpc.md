File_Code/solana/578f2aa22b/rpc/rpc_after.rs --- Rust
1067         let last_element = blocks.last().cloned().unwrap_or_default();                                                                                  1067         let last_element = blocks
                                                                                                                                                             1068             .last()
                                                                                                                                                             1069             .cloned()
                                                                                                                                                             1070             .unwrap_or_else(|| start_slot.saturating_sub(1));

