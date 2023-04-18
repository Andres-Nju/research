File_Code/rust/c788433b15/builder/builder_after.rs --- Rust
388         if paths[0] == Path::new("nonexistent/path/to/trigger/cargo/metadata") {                                                                         388         if let Some(path) = paths.get(0) {
...                                                                                                                                                          389             if path == Path::new("nonexistent/path/to/trigger/cargo/metadata") {
389             return;                                                                                                                                      390                 return;
390         }                                                                                                                                                391             }
                                                                                                                                                             392         }

