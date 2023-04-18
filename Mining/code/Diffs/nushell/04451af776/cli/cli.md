File_Code/nushell/04451af776/cli/cli_after.rs --- Rust
758                     .map(|s| s.value.expect_string() == "true")                                                                                          758                     .map(|s| s.value.is_true())

