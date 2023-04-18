File_Code/parity-ethereum/319a01b34f/blockchain/blockchain_after.rs --- Rust
534         let user_defaults = UserDefaults::load(&user_defaults_path)?;                                                                                    534         let mut user_defaults = UserDefaults::load(&user_defaults_path)?;
535         let algorithm = cmd.pruning.to_algorithm(&user_defaults);                                                                                        535         let algorithm = cmd.pruning.to_algorithm(&user_defaults);
536         let dir = db_dirs.db_path(algorithm);                                                                                                            536         let dir = db_dirs.db_path(algorithm);
537         fs::remove_dir_all(&dir).map_err(|e| format!("Error removing database: {:?}", e))?;                                                              537         fs::remove_dir_all(&dir).map_err(|e| format!("Error removing database: {:?}", e))?;
                                                                                                                                                             538         user_defaults.is_first_launch = true;
                                                                                                                                                             539         user_defaults.save(&user_defaults_path)?;

