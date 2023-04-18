File_Code/solana/ad159e0906/fullnode/fullnode_after.rs --- Rust
121             let balance = client.poll_get_balance(&leader_pubkey).unwrap();                                                                              121             let balance = client.poll_get_balance(&leader_pubkey).unwrap_or(0);

