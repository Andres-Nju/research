File_Code/solana/c6695a3120/lib/lib_after.rs --- 1/2 --- Rust
78             bincode::deserialize(&data[3..]).map_err(|err| {                                                                                              78             bincode::deserialize(&data[4..]).map_err(|err| {

File_Code/solana/c6695a3120/lib/lib_after.rs --- 2/2 --- Rust
 ..                                                                                                                                                          484         assert_eq!(
484         get_public_ip_addr(&ip_echo_server_addr).unwrap();                                                                                               485             get_public_ip_addr(&ip_echo_server_addr),
                                                                                                                                                             486             parse_host("127.0.0.1"),
                                                                                                                                                             487         );

