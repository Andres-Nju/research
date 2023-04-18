File_Code/rust/e5d1b9ca39/lib/lib_after.rs --- Rust
390                     span_bug!(span, "Could not find container for method {}", id);                                                                       390                     debug!("Could not find container for method {} at {:?}", id, span);
                                                                                                                                                             391                     // This is not necessarily a bug, if there was a compilation error, the tables
                                                                                                                                                             392                     // we need might not exist.
                                                                                                                                                             393                     return None;

