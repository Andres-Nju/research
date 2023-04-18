File_Code/rust/16fc74c6a2/build/build_after.rs --- 1/2 --- Rust
114     if cfg!(feature = "debug") {                                                                                                                         114     // FIXME: building with jemalloc assertions is currently broken.
115         cmd.arg("--enable-debug");                                                                                                                       115     // See <https://github.com/rust-lang/rust/issues/44152>.
116     }                                                                                                                                                    116     //if cfg!(feature = "debug") {

File_Code/rust/16fc74c6a2/build/build_after.rs --- 2/2 --- Rust
114     if cfg!(feature = "debug") {                                                                                                                         114     // FIXME: building with jemalloc assertions is currently broken.
115         cmd.arg("--enable-debug");                                                                                                                       115     // See <https://github.com/rust-lang/rust/issues/44152>.
116     }                                                                                                                                                    116     //if cfg!(feature = "debug") {
                                                                                                                                                             117     //    cmd.arg("--enable-debug");
                                                                                                                                                             118     //}

