File_Code/solana/15aa07f2a0/broadcast_stage/broadcast_stage_after.rs --- 1/2 --- Rust
80         // Layer 1, leader nodes are limited to the fanout size.                                                                                            
81         broadcast_table.truncate(NEIGHBORHOOD_SIZE);                                                                                                        

File_Code/solana/15aa07f2a0/broadcast_stage/broadcast_stage_after.rs --- 2/2 --- Rust
                                                                                                                                                             81         // Layer 1, leader nodes are limited to the fanout size.
                                                                                                                                                             82         broadcast_table.truncate(NEIGHBORHOOD_SIZE);

