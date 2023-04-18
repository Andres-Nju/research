File_Code/cargo/54159e84ce/registry/registry_after.rs --- Rust
347         let boilerplate = "\                                                                                                                             347         let boilerplate = "\
348 This is currently allowed but is known to produce buggy behavior with spurious                                                                           348 This is currently allowed but is known to produce buggy behavior with spurious
349 recompiles and changes to the crate graph. Path overrides unfortunately were                                                                             349 recompiles and changes to the crate graph. Path overrides unfortunately were
350 never intended to support this feature, so for now this message is just a                                                                                350 never intended to support this feature, so for now this message is just a
351 warning. In the future, however, this message will become a hard error.                                                                                  351 warning. In the future, however, this message will become a hard error.
352                                                                                                                                                          352 
353 To change the dependency graph via an override it's recommended to use the                                                                               353 To change the dependency graph via an override it's recommended to use the
354 `[replace]` feature of Cargo instead of the path override feature. This is                                                                               354 `[replace]` feature of Cargo instead of the path override feature. This is
355 documented online at the url below for more information.                                                                                                 355 documented online at the url below for more information.
356                                                                                                                                                          356 
357 http://doc.crates.io/specifying-dependencies.html#overriding-dependencies                                                                                357 https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#overriding-dependencies
358 ";                                                                                                                                                       358 ";

