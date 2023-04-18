File_Code/servo/5034d1d777/scaffolding/scaffolding_after.rs --- 1/2 --- Rust
13     let top = Path::new(file!()).parent().unwrap().join("..").join("..").join("..").join("..");                                                           13     let top = Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap()).join("..").join("..").join("..");

File_Code/servo/5034d1d777/scaffolding/scaffolding_after.rs --- 2/2 --- Rust
27     assert!(status.success());                                                                                                                            27     assert!(status.success(), "{:?}", status);

