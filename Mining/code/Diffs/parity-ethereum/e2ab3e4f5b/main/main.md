File_Code/parity-ethereum/e2ab3e4f5b/main/main_after.rs --- Rust
193         let logger = setup_log(&conf.logger_config()).expect("Logger is initialized only once; qed");                                                    193         let logger = setup_log(&conf.logger_config()).unwrap_or_else(|e| {
                                                                                                                                                             194                 eprintln!("{}", e);
                                                                                                                                                             195                 process::exit(2)
                                                                                                                                                             196         });

