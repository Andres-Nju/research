File_Code/rust/e9e4836532/build/build_after.rs --- 1/2 --- Rust
                                                                                                                                                            59         let target_endian_little = env::var("CARGO_CFG_TARGET_ENDIAN").unwrap() != "big";

File_Code/rust/e9e4836532/build/build_after.rs --- 2/2 --- Rust
                                                                                                                                                            66         // libunwind expects a __LITTLE_ENDIAN__ macro to be set for LE archs, cf. #65765
                                                                                                                                                            67         if target_endian_little {
                                                                                                                                                            68             cfg.define("__LITTLE_ENDIAN__", Some("1"));
                                                                                                                                                            69         }

