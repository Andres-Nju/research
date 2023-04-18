File_Code/rust/3545fb512e/rustdoc/rustdoc_after.rs --- Rust
                                                                                                                                                            44     // Pass the `rustbuild` feature flag to crates which rustbuild is
                                                                                                                                                            45     // building. See the comment in bootstrap/lib.rs where this env var is
                                                                                                                                                            46     // set for more details.
                                                                                                                                                            47     if env::var_os("RUSTBUILD_UNSTABLE").is_some() {
                                                                                                                                                            48         cmd.arg("--cfg").arg("rustbuild");
                                                                                                                                                            49     }

