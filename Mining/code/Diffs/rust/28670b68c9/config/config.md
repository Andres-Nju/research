File_Code/rust/28670b68c9/config/config_after.rs --- Rust
                                                                                                                                                          1973     if debugging_opts.profile && incremental.is_some() {
                                                                                                                                                          1974         early_error(
                                                                                                                                                          1975             error_format,
                                                                                                                                                          1976             "can't instrument with gcov profiling when compiling incrementally",
                                                                                                                                                          1977         );
                                                                                                                                                          1978     }

