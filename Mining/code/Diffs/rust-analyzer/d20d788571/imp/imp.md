File_Code/rust-analyzer/d20d788571/imp/imp_after.rs --- 1/2 --- Rust
46         self.gc_syntax_trees();                                                                                                                           46         // self.gc_syntax_trees();

File_Code/rust-analyzer/d20d788571/imp/imp_after.rs --- 2/2 --- Rust
                                                                                                                                                            120     #[allow(unused)]
                                                                                                                                                            121     /// Ideally, we should call this function from time to time to collect heavy
                                                                                                                                                            122     /// syntax trees. However, if we actually do that, everything is recomputed
                                                                                                                                                            123     /// for some reason. Needs investigation.

