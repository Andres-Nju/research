File_Code/solana/87c0d8d9e7/command/command_after.rs --- Rust
439     let shell_export_string = format!(r#"export PATH="{}:$PATH""#, new_path);                                                                            439     let shell_export_string = format!("\nexport PATH=\"{}:$PATH\"", new_path);

