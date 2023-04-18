File_Code/cargo/9f4330d6c1/mod/mod_after.rs --- Rust
261                 try!(fs::remove_file(&dst));                                                                                                             261                 try!(fs::remove_file(&dst).chain_error(|| {
                                                                                                                                                             262                     human(format!("Could not remove file: {}.", dst.display()))
                                                                                                                                                             263                 }));

