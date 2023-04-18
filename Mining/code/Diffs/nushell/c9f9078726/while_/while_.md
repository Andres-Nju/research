File_Code/nushell/c9f9078726/while_/while__after.rs --- 1/2 --- Rust
                                                                                                                                                             1 use std::sync::atomic::Ordering;

File_Code/nushell/c9f9078726/while_/while__after.rs --- 2/2 --- Rust
                                                                                                                                                            51             if let Some(ctrlc) = &engine_state.ctrlc {
                                                                                                                                                            52                 if ctrlc.load(Ordering::SeqCst) {
                                                                                                                                                            53                     break;
                                                                                                                                                            54                 }
                                                                                                                                                            55             }

