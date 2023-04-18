File_Code/solana/40b7c11262/cli/cli_after.rs --- 1/2 --- Rust
849                 matches.value_of("base85_transaction").unwrap().to_string(),                                                                             849                 matches.value_of("base58_transaction").unwrap().to_string(),

File_Code/solana/40b7c11262/cli/cli_after.rs --- 2/2 --- Rust
2437                 .about("Decode a base-85 binary transaction")                                                                                           2437                 .about("Decode a base-58 binary transaction")
2438                 .arg(                                                                                                                                   2438                 .arg(
2439                     Arg::with_name("base85_transaction")                                                                                                2439                     Arg::with_name("base58_transaction")

