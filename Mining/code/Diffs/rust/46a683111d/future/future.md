File_Code/rust/46a683111d/future/future_after.rs --- Text (2 errors, exceeded DFT_PARSE_ERROR_LIMIT)
 98     let mut waker_ptr = waker_ptr.expect(                                                                                                                 98     let waker_ptr = waker_ptr.expect(
 99         "TLS LocalWaker not set. This is a rustc bug. \                                                                                                   99         "TLS LocalWaker not set. This is a rustc bug. \
100         Please file an issue on https://github.com/rust-lang/rust.");                                                                                    100         Please file an issue on https://github.com/rust-lang/rust.");
101     unsafe { f(waker_ptr.as_mut()) }                                                                                                                     101     unsafe { f(waker_ptr.as_ref()) }

