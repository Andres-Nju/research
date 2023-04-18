File_Code/rust/a516dbb7d9/ops/ops_after.rs --- Rust
                                                                                                                                                           104     ///
                                                                                                                                                           105     /// This function cannot be called explicitly. This is compiler error
                                                                                                                                                           106     /// [0040]. However, the [`std::mem::drop`] function in the prelude can be
                                                                                                                                                           107     /// used to call the argument's `Drop` implementation.
                                                                                                                                                           108     ///
                                                                                                                                                           109     /// [0040]: https://doc.rust-lang.org/error-index.html#E0040
                                                                                                                                                           110     /// [`std::mem::drop`]: https://doc.rust-lang.org/std/mem/fn.drop.html

