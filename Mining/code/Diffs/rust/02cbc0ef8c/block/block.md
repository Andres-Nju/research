File_Code/rust/02cbc0ef8c/block/block_after.rs --- Rust
897         let llscratch = bcx.alloca(llcast_ty, "fn_ret_cast");                                                                                            897         let llscratch = bcx.with_block(|bcx| {
...                                                                                                                                                          898             let alloca = base::alloca(bcx, llcast_ty, "fn_ret_cast");
898         bcx.with_block(|bcx| base::call_lifetime_start(bcx, llscratch));                                                                                 899             base::call_lifetime_start(bcx, alloca);
                                                                                                                                                             900             alloca
                                                                                                                                                             901         });

