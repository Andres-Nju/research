File_Code/rust-analyzer/0010d62cad/lib/lib_after.rs --- Rust
129         let mut path = std::env::current_exe().unwrap();                                                                                                 129         let path = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
130         while !path.join("Cargo.toml").is_file() {                                                                                                           
131             path = path.parent().unwrap().to_owned();                                                                                                        
132         }                                                                                                                                                    

