File_Code/solana/8d0d429acd/bench-tps/bench-tps_after.rs --- Rust
147                 .poll_get_balance(&id.pubkey())                                                                                                          147                 .poll_balance_with_timeout(
                                                                                                                                                             148                     &id.pubkey(),
                                                                                                                                                             149                     &Duration::from_millis(100),
                                                                                                                                                             150                     &Duration::from_secs(10),

