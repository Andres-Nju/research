File_Code/solana/eec3d25ab9/cli/cli_after.rs --- Rust
1274         log_instruction_custom_error::<SystemError>(result, &config).map_err(|_| {                                                                      1274         log_instruction_custom_error::<SystemError>(result, &config).map_err(|err| {
1275             CliError::DynamicProgramError("Program account allocation failed".to_string())                                                              1275             CliError::DynamicProgramError(format!("Program account allocation failed: {}", err))

