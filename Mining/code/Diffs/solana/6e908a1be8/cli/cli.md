File_Code/solana/6e908a1be8/cli/cli_after.rs --- Rust
1343         .map_err(|_| {                                                                                                                                  1343         .map_err(|e| {
1344             CliError::DynamicProgramError("Program finalize transaction failed".to_string())                                                            1344             CliError::DynamicProgramError(format!("Program finalize transaction failed: {}", e))

