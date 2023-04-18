File_Code/rust/347a42e387/lib/lib_after.rs --- Rust
 .                                                                                                                                                           61         // call std::sys::abort_internal
61         extern "C" { pub fn panic_exit() -> !; }                                                                                                          62         extern "C" { pub fn __rust_abort() -> !; }
62         panic_exit();                                                                                                                                     63         __rust_abort();

