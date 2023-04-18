File_Code/rust/a399326ca2/lto/lto_after.rs --- Rust
616         assert!(!llmod.is_null());                                                                                                                       616         if llmod.is_null() {
                                                                                                                                                             617             let msg = format!("failed to parse bitcode for thin LTO module");
                                                                                                                                                             618             return Err(write::llvm_err(&diag_handler, msg));
                                                                                                                                                             619         }

