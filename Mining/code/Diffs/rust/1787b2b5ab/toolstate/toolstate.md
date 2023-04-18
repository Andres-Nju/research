File_Code/rust/1787b2b5ab/toolstate/toolstate_after.rs --- Rust
416     file.insert_str(end_of_first_line, &format!("{}\t{}\n", commit, toolstate_serialized));                                                              416     file.insert_str(end_of_first_line, &format!("\n{}\t{}", commit, toolstate_serialized));

