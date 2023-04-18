File_Code/alacritty/92ea355eee/mod/mod_after.rs --- 1/2 --- Rust
11 use libc::c_uint;                                                                                                                                         11 use libc::{c_long, c_uint};

File_Code/alacritty/92ea355eee/mod/mod_after.rs --- 2/2 --- Rust
92 fn to_fixedpoint_16_6(f: f64) -> i64 {                                                                                                                    92 fn to_fixedpoint_16_6(f: f64) -> c_long {
93     (f * 65536.0) as i64                                                                                                                                  93     (f * 65536.0) as c_long

