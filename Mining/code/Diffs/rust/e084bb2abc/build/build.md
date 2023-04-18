File_Code/rust/e084bb2abc/build/build_after.rs --- Rust
44     } else if target.contains("apple-darwin") {                                                                                                           44     } else if target.contains("solaris") {
                                                                                                                                                             45         println!("cargo:rustc-link-lib=socket");
                                                                                                                                                             46         println!("cargo:rustc-link-lib=posix4");
                                                                                                                                                             47         println!("cargo:rustc-link-lib=pthread");
                                                                                                                                                             48     } else if target.contains("apple-darwin") {

