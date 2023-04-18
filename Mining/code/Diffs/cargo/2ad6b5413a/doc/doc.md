File_Code/cargo/2ad6b5413a/doc/doc_after.rs --- 1/2 --- Rust
1317             "\                                                                                                                                          1317             "\
1318 [WARNING] `[bad_link]` cannot be resolved, ignoring it...                                                                                               1318 [WARNING] `[bad_link]` cannot be resolved[..]
1319 ",                                                                                                                                                      1319 ",

File_Code/cargo/2ad6b5413a/doc/doc_after.rs --- 2/2 --- Rust
1363         .with_stderr_contains(                                                                                                                          1363         .with_stderr_contains("src/lib.rs:4:6: error: `[bad_link]` cannot be resolved[..]")
1364             "src/lib.rs:4:6: error: `[bad_link]` cannot be resolved, ignoring it...",                                                                        

