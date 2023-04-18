File_Code/rust/860c6ab3c1/check/check_after.rs --- Rust
245     let output = testdir(build, compiler.host).join("error-index.md");                                                                                   245     let dir = testdir(build, compiler.host);
                                                                                                                                                             246     t!(fs::create_dir_all(&dir));
                                                                                                                                                             247     let output = dir.join("error-index.md");

