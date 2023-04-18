File_Code/solana/d87910eb15/ip_echo_server/ip_echo_server_after.rs --- Rust
15         TcpListener::bind(&bind_addr).unwrap_or_else(|_| panic!("Unable to bind to {}", bind_addr));                                                      15         .unwrap_or_else(|err| panic!("Unable to bind to {}: {}", bind_addr, err));

