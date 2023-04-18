File_Code/rust/a5193057b9/util/util_after.rs --- Rust
                                                                                                                                                            44     // A call to `hard_link` will fail if `dst` exists, so remove it if it
                                                                                                                                                            45     // already exists so we can try to help `hard_link` succeed.
                                                                                                                                                            46     let _ = fs::remove_file(&dst);
                                                                                                                                                            47 
                                                                                                                                                            48     // Attempt to "easy copy" by creating a hard link (symlinks don't work on
                                                                                                                                                            49     // windows), but if that fails just fall back to a slow `copy` operation.

