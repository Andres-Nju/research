File_Code/rust/e412cb30dc/lto/lto_after.rs --- 1/3 --- Rust
                                                                                                                                                            30 use std::ptr::read_unaligned;

File_Code/rust/e412cb30dc/lto/lto_after.rs --- 2/3 --- Rust
226     let data = unsafe { *(byte_data.as_ptr() as *const u32) };                                                                                           227     let data = unsafe { read_unaligned(byte_data.as_ptr() as *const u32) };

File_Code/rust/e412cb30dc/lto/lto_after.rs --- 3/3 --- Rust
233     let data = unsafe { *(byte_data.as_ptr() as *const u64) };                                                                                           234     let data = unsafe { read_unaligned(byte_data.as_ptr() as *const u64) };

