File_Code/rust/ebbcad9ae0/compile/compile_after.rs --- 1/2 --- Rust
32 use util::{exe, libdir, is_dylib, copy, read_stamp_file};                                                                                                 32 use util::{exe, libdir, is_dylib, copy, read_stamp_file, CiEnv};

File_Code/rust/ebbcad9ae0/compile/compile_after.rs --- 2/2 --- Rust
795     if stderr_isatty() {                                                                                                                                 795     if stderr_isatty() && build.ci_env == CiEnv::None {

