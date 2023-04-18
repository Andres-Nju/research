File_Code/solana/effcef2184/crdt/crdt_after.rs --- 1/2 --- Rust
                                                                                                                                                           831     use std::thread::sleep;

File_Code/solana/effcef2184/crdt/crdt_after.rs --- 2/2 --- Rust
                                                                                                                                                          1091         while now == crdt.alive[&nxt2.id] {
                                                                                                                                                          1092             sleep(Duration::from_millis(GOSSIP_SLEEP_MILLIS));
                                                                                                                                                          1093             crdt.insert(&nxt2);
                                                                                                                                                          1094         }

