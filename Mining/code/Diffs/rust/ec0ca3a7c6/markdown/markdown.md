File_Code/rust/ec0ca3a7c6/markdown/markdown_after.rs --- 1/2 --- Rust
531     fn hoedown_buffer_puts(b: *mut hoedown_buffer, c: *const libc::c_char);                                                                                

File_Code/rust/ec0ca3a7c6/markdown/markdown_after.rs --- 2/2 --- Rust
632         unsafe { hoedown_buffer_puts(ob, "\n\0".as_ptr() as *const _); }                                                                                 631         unsafe { hoedown_buffer_put(ob, "\n".as_ptr(), 1); }

