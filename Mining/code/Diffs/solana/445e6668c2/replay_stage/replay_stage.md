File_Code/solana/445e6668c2/replay_stage/replay_stage_after.rs --- Rust
499         let empty = HashSet::new();                                                                                                                      499         let slot_descendants = descendants.get(&duplicate_slot);
500         let slot_descendants = descendants.get(&duplicate_slot).unwrap_or(&empty);                                                                       500         if slot_descendants.is_none() {
...                                                                                                                                                          501             // Root has already moved past this slot, no need to purge it
...                                                                                                                                                          502             return;
...                                                                                                                                                          503         }
501                                                                                                                                                          504 
502         for d in slot_descendants                                                                                                                        505         for d in slot_descendants
                                                                                                                                                             506             .unwrap()

