File_Code/solana/95490ff56e/build/build_after.rs --- Rust
                                                                                                                                                            20             // See https://github.com/solana-labs/solana/issues/11055
                                                                                                                                                            21             // We may be running the custom `rust-bpf-builder` toolchain,
                                                                                                                                                            22             // which currently needs `#![feature(proc_macro_hygiene)]` to
                                                                                                                                                            23             // be applied.
                                                                                                                                                            24             println!("cargo:rustc-cfg=RUSTC_NEEDS_PROC_MACRO_HYGIENE");

