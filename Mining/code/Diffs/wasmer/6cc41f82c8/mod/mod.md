File_Code/wasmer/6cc41f82c8/mod/mod_after.rs --- Rust
444     let ret = unsafe { lseek(fd, offset, whence) };                                                                                                      444     let ret = unsafe { lseek(fd, offset, whence) as i64 };

