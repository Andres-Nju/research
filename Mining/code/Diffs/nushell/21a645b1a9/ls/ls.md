File_Code/nushell/21a645b1a9/ls/ls_after.rs --- Rust
  .                                                                                                                                                          802                     format!(
  .                                                                                                                                                          803                         "Could not read metadata for '{}'. It may have an illegal filename.",
802                     "Could not read file metadata".to_string(),                                                                                          804                         filename.to_string_lossy()
                                                                                                                                                             805                     ),

