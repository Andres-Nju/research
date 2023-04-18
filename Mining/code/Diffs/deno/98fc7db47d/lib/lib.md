File_Code/deno/98fc7db47d/lib/lib_after.rs --- 1/2 --- Rust
127   println!(                                                                                                                                              127   let types = format!(

File_Code/deno/98fc7db47d/lib/lib_after.rs --- 2/2 --- Rust
                                                                                                                                                             133   use std::io::Write;
                                                                                                                                                             134   let _r = std::io::stdout().write_all(types.as_bytes());
                                                                                                                                                             135   // TODO(ry) Only ignore SIGPIPE. Currently ignoring all errors.

