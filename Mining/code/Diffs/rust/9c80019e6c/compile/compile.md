File_Code/rust/9c80019e6c/compile/compile_after.rs --- Rust
                                                                                                                                                          1010             if build.config.rustc_error_format.as_ref().map_or(false, |e| e == "json") {
                                                                                                                                                          1011                 // most likely not a cargo message, so let's send it out as well
                                                                                                                                                          1012                 println!("{}", line);
                                                                                                                                                          1013             }

