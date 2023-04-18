File_Code/deno/f07f246ae8/repl_tests/repl_tests_after.rs --- Rust
  .                                                                                                                                                          166     if cfg!(windows) {
166     assert!(output.contains("testing output\u{1b}"));                                                                                                    167       assert!(output.contains("testing output\u{1b}"));
                                                                                                                                                             168     } else {
                                                                                                                                                             169       assert!(output.contains("\ntesting output"));
                                                                                                                                                             170     }

