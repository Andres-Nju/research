File_Code/deno/9830ae8297/lib/lib_after.rs --- Rust
757   let tx = ctx.requests.get_mut(&token).unwrap();                                                                                                        757   let tx = match ctx.requests.get_mut(&token) {
                                                                                                                                                             758     Some(tx) => tx,
                                                                                                                                                             759     // request was already consumed by caller
                                                                                                                                                             760     None => return 0,
                                                                                                                                                             761   };

