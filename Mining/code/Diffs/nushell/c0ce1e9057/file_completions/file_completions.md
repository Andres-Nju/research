File_Code/nushell/c0ce1e9057/file_completions/file_completions_after.rs --- Rust
                                                                                                                                                           139                         // Fix files or folders with quotes
                                                                                                                                                           140                         if path.contains('\'') || path.contains('"') {
                                                                                                                                                           141                             path = format!("`{}`", path);
                                                                                                                                                           142                         }

