File_Code/starship/7b9197a67e/python/python_after.rs --- Rust
                                                                                                                                                            71             if !output.status.success() {
                                                                                                                                                            72                 log::warn!(
                                                                                                                                                            73                     "Non-Zero exit code '{}' when executing `python --version`",
                                                                                                                                                            74                     output.status
                                                                                                                                                            75                 );
                                                                                                                                                            76                 return None;
                                                                                                                                                            77             }

