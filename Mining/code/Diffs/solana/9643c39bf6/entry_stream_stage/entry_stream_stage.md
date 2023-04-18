File_Code/solana/9643c39bf6/entry_stream_stage/entry_stream_stage_after.rs --- 1/3 --- Rust
 .                                                                                                                                                           73                 let block_slot = queued_block.unwrap().slot;
73                 let block_tick_height = queued_block.unwrap().tick_height;                                                                                74                 let block_tick_height = queued_block.unwrap().tick_height;
74                 let block_id = queued_block.unwrap().id;                                                                                                  75                 let block_id = queued_block.unwrap().id;
75                 entry_stream                                                                                                                              76                 entry_stream
76                     .emit_block_event(slot, &leader_id, block_tick_height, block_id)                                                                      77                     .emit_block_event(block_slot, &leader_id, block_tick_height, block_id)

File_Code/solana/9643c39bf6/entry_stream_stage/entry_stream_stage_after.rs --- 2/3 --- Rust
                                                                                                                                                             90                     slot,

File_Code/solana/9643c39bf6/entry_stream_stage/entry_stream_stage_after.rs --- 3/3 --- Rust
                                                                                                                                                            188             let slot = json["s"].as_u64().unwrap();
                                                                                                                                                            189             assert_eq!(0, slot);

