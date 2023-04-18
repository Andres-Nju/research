File_Code/solana/71bf8c5f85/keygen/keygen_after.rs --- 1/2 --- Rust
240             let ignore_case = matches.is_present("ignore-case");                                                                                         240             let ignore_case = matches.is_present("ignore_case");
241             let includes = if matches.is_present("includes") {                                                                                           241             let includes = if matches.is_present("includes") {
242                 values_t_or_exit!(matches, "includes", String)                                                                                           242                 values_t_or_exit!(matches, "includes", String)
243                     .into_iter()                                                                                                                         243                     .into_iter()
                                                                                                                                                             244                     .map(|s| if ignore_case { s.to_lowercase() } else { s })

File_Code/solana/71bf8c5f85/keygen/keygen_after.rs --- 2/2 --- Rust
                                                                                                                                                             253                     .map(|s| if ignore_case { s.to_lowercase() } else { s })

