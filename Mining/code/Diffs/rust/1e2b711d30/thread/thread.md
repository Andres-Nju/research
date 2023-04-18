File_Code/rust/1e2b711d30/thread/thread_after.rs --- 1/3 --- Rust
34         const CLOCK_ID: wasi::Userdata = 0x0123_45678;                                                                                                    34         const USERDATA: wasi::Userdata = 0x0123_45678;
35                                                                                                                                                           35 
36         let clock = wasi::raw::__wasi_subscription_u_clock_t {                                                                                            36         let clock = wasi::raw::__wasi_subscription_u_clock_t {
37             identifier: CLOCK_ID,                                                                                                                         37             identifier: 0,

File_Code/rust/1e2b711d30/thread/thread_after.rs --- 2/3 --- Rust
45             userdata: 0,                                                                                                                                  45             userdata: USERDATA,

File_Code/rust/1e2b711d30/thread/thread_after.rs --- 3/3 --- Rust
56                 userdata: CLOCK_ID,                                                                                                                       56                 userdata: USERDATA,

