File_Code/solana/37b048effb/main/main_after.rs --- 1/2 --- Rust
1008                 for port in &[rpc_port, rpc_pubsub_port] {                                                                                              1008                 for (purpose, port) in &[("RPC", rpc_port), ("RPC pubsub", rpc_pubsub_port)] {

File_Code/solana/37b048effb/main/main_after.rs --- 2/2 --- Rust
1013                                 error!("Unable to bind to tcp/{}: {}", port, err);                                                                      1013                                 error!("Unable to bind to tcp/{} for {}: {}", port, purpose, err);

