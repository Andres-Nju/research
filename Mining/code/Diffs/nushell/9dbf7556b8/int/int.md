File_Code/nushell/9dbf7556b8/int/int_after.rs --- Rust
194             error: ShellError::UnsupportedInput("'into int' for unsupported type".into(), span),                                                         194             error: ShellError::UnsupportedInput(
                                                                                                                                                             195                 format!("'into int' for unsupported type '{}'", input.get_type()),
                                                                                                                                                             196                 span,

