File_Code/rust/5aa29c4c82/const_eval/const_eval_after.rs --- Rust
                                                                                                                                                           342         let alloc = ecx
                                                                                                                                                           343                     .tcx
                                                                                                                                                           344                     .interpret_interner
                                                                                                                                                           345                     .get_cached(cid.instance.def_id());
                                                                                                                                                           346         // Don't evaluate when already cached to prevent cycles
                                                                                                                                                           347         if let Some(alloc) = alloc {
                                                                                                                                                           348             return Ok(alloc)
                                                                                                                                                           349         }

