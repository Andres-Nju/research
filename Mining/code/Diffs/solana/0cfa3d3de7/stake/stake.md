File_Code/solana/0cfa3d3de7/stake/stake_after.rs --- Rust
600     let stake_history = StakeHistory::from_account(&stake_history_account).unwrap();                                                                     600     let stake_history = StakeHistory::from_account(&stake_history_account).ok_or_else(|| {
                                                                                                                                                             601         CliError::RpcRequestError("Failed to deserialize stake history".to_string())
                                                                                                                                                             602     })?;

