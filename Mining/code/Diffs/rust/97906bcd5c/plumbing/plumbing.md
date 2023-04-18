File_Code/rust/97906bcd5c/plumbing/plumbing_after.rs --- Rust
                                                                                                                                                           328         // Be careful reyling on global state here: this code is called from
                                                                                                                                                           329         // a panic hook, which means that the global `Handler` may be in a weird
                                                                                                                                                           330         // state if it was responsible for triggering the panic.

