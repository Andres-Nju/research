File_Code/solana/2cb500a388/main/main_after.rs --- 1/2 --- Rust
2205                     skip_rewrites: matches.is_present("accounts_db_skip_rewrites"),                                                                     2205                     skip_rewrites: arg_matches.is_present("accounts_db_skip_rewrites"),
2206                     ancient_append_vecs: matches.is_present("accounts_db_ancient_append_vecs"),                                                         2206                     ancient_append_vecs: arg_matches.is_present("accounts_db_ancient_append_vecs"),
2207                     skip_initial_hash_calc: matches                                                                                                     2207                     skip_initial_hash_calc: arg_matches

File_Code/solana/2cb500a388/main/main_after.rs --- 2/2 --- Rust
2230                         bpf_jit: !matches.is_present("no_bpf_jit"),                                                                                     2230                         bpf_jit: !arg_matches.is_present("no_bpf_jit"),

