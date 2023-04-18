File_Code/rust/5fbcf08dd8/eh/eh_after.rs --- Rust
111         // If ip is not present in the table, call terminate.  This is for                                                                               111         // Ip is not present in the table.  This should not hapen... but it does: issie #35011.
112         // a destructor inside a cleanup, or a library routine the compiler                                                                              112         // So rather than returning EHAction::Terminate, we do this.
113         // was not expecting to throw                                                                                                                    ... 
114         EHAction::Terminate                                                                                                                              113         EHAction::None

