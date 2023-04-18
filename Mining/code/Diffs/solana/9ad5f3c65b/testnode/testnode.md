File_Code/solana/9ad5f3c65b/testnode/testnode_after.rs --- Rust
132     let threads = if matches.opt_present("r") {                                                                                                          132     let threads = if matches.opt_present("v") {
133         eprintln!("starting validator... {}", repl_data.requests_addr);                                                                                  133         eprintln!("starting validator... {}", repl_data.requests_addr);
134         let path = matches.opt_str("r").unwrap();                                                                                                        134         let path = matches.opt_str("v").unwrap();

