File_Code/parity-ethereum/330cdc1a4d/cluster_connections_net/cluster_connections_net_after.rs --- 1/2 --- Rust
470         let now = Instant::now();                                                                                                                          

File_Code/parity-ethereum/330cdc1a4d/cluster_connections_net/cluster_connections_net_after.rs --- 2/2 --- Rust
...                                                                                                                                                          472                 // the last_message_time could change after active_connections() call
...                                                                                                                                                          473                 // => we always need to call Instant::now() after getting last_message_time
473                 let last_message_diff = now - connection.last_message_time();                                                                            474                 let last_message_time = connection.last_message_time();
                                                                                                                                                             475                 let now = Instant::now();
                                                                                                                                                             476                 let last_message_diff = now - last_message_time;

