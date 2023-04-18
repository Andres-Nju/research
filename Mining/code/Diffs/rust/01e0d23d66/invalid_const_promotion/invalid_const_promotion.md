File_Code/rust/01e0d23d66/invalid_const_promotion/invalid_const_promotion_after.rs --- Rust
42             || status.signal() == Some(libc::SIGABRT));                                                                                                   42             || status.signal() == Some(libc::SIGTRAP)
                                                                                                                                                             43             || status.signal() == Some(libc::SIGABRT));

