File_Code/solana/85c9a231c1/blocktree/blocktree_after.rs --- 1/2 --- Rust
745                         "Received index {} >= slot.last_index {}",                                                                                       745                         "Slot {}: received index {} >= slot.last_index {}",
746                         shred_index, last_index                                                                                                          746                         slot, shred_index, last_index

File_Code/solana/85c9a231c1/blocktree/blocktree_after.rs --- 2/2 --- Rust
761                         "Received shred_index {} < slot.received {}",                                                                                    761                         "Slot {}: received shred_index {} < slot.received {}",
762                         shred_index, slot_meta.received                                                                                                  762                         slot, shred_index, slot_meta.received

