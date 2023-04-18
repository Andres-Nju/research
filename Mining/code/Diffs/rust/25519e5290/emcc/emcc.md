File_Code/rust/25519e5290/emcc/emcc_after.rs --- Rust
91         ptr::drop_in_place(ptr as *mut Exception);                                                                                                        91         if let Some(b) = (ptr as *mut Exception).read().data {
..                                                                                                                                                           92             drop(b);
..                                                                                                                                                           93             super::__rust_drop_panic();
..                                                                                                                                                           94         }
92         super::__rust_drop_panic();                                                                                                                       95         #[cfg(any(target_arch = "arm", target_arch = "wasm32"))]
                                                                                                                                                             96         ptr

