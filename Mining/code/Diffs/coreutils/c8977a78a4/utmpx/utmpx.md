File_Code/coreutils/c8977a78a4/utmpx/utmpx_after.rs --- Rust
245         // FixME: discuss and revise a rewrite which is correct and satisfies clippy/rustc                                                                 . 
246         #[allow(clippy::temporary_cstring_as_ptr)]                                                                                                         . 
247         let res = unsafe { utmpxname(CString::new(f).unwrap().as_ptr()) };                                                                               245         let res = unsafe {
                                                                                                                                                             246             let cstr = CString::new(f).unwrap();
                                                                                                                                                             247             utmpxname(cstr.as_ptr())

