File_Code/solana/965ad778dd/main/main_after.rs --- Text (4 errors, exceeded DFT_PARSE_ERROR_LIMIT)
113                         return Err(CliError::KeypairFileNotFound(                                                                                        113                         return Err(CliError::KeypairFileNotFound(format!(
114                             "Generate a new keypair with `solana-keygen new`".to_string(),                                                               114                             "Generate a new keypair at {} with `solana-keygen new`",
115                         )                                                                                                                                115                             default_keypair_path
                                                                                                                                                             116                         ))

