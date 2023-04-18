File_Code/rust/66237afce4/rt/rt_after.rs --- 1/2 --- Rust
                                                                                                                                                            38     #[cfg(not(feature = "backtrace"))]
                                                                                                                                                            39     use mem;

File_Code/rust/66237afce4/rt/rt_after.rs --- 2/2 --- Rust
 .                                                                                                                                                           58         #[cfg(feature = "backtrace")]
56         let res = panic::catch_unwind(|| {                                                                                                                59         let res = panic::catch_unwind(|| {
57             ::sys_common::backtrace::__rust_begin_short_backtrace(main)                                                                                   60             ::sys_common::backtrace::__rust_begin_short_backtrace(main)
58         });                                                                                                                                               61         });
                                                                                                                                                             62         #[cfg(not(feature = "backtrace"))]
                                                                                                                                                             63         let res = panic::catch_unwind(mem::transmute::<_, fn()>(main));

