File_Code/rust/99b6247166/consider-removing-last-semi/consider-removing-last-semi_after.rs --- Rust
                                                                                                                                                            17 fn g() -> String {  //~ ERROR E0269
                                                                                                                                                            18                     //~^ HELP detailed explanation
                                                                                                                                                            19     "this won't work".to_string();
                                                                                                                                                            20     "removeme".to_string(); //~ HELP consider removing this semicolon
                                                                                                                                                            21 }

