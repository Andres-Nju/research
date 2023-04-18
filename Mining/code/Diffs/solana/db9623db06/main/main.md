File_Code/solana/db9623db06/main/main_after.rs --- Rust
33     let index = args.iter().position(|x| x == "--").unwrap_or(args.len());                                                                                  
34     args.insert(index, "bpf".to_string());                                                                                                                  
35     args.insert(index, "--arch".to_string());                                                                                                               

