File_Code/nushell/457f7889df/exists/exists_after.rs --- Rust
108         val: path.exists(),                                                                                                                              108         val: match path.try_exists() {
                                                                                                                                                             109             Ok(exists) => exists,
                                                                                                                                                             110             Err(err) => return Value::Error { error: err.into() },
                                                                                                                                                             111         },

