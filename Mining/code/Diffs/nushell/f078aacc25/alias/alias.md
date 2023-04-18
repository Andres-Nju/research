File_Code/nushell/f078aacc25/alias/alias_after.rs --- Rust
192                     _ => Err(ShellError::labeled_error(                                                                                                  192                     _ => Err(ShellError::labeled_error_with_secondary(
193                         "Type conflict in alias variable use",                                                                                           193                         "Type conflict in alias variable use",
194                         "creates type conflict",                                                                                                         194                         format!("{:?}", new),
195                         (to_add.1).0,                                                                                                                    195                         (to_add.1).0,
                                                                                                                                                             196                         format!("{:?}", shape),
                                                                                                                                                             197                         exist.0,

