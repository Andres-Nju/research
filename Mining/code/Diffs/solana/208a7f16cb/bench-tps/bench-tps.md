File_Code/solana/208a7f16cb/bench-tps/bench-tps_after.rs --- Rust
278     let starting_balance = client.poll_get_balance(&id.pubkey()).unwrap();                                                                               278     let starting_balance = client.poll_get_balance(&id.pubkey()).unwrap_or(0);

