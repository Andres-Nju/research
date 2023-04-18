File_Code/rust/c661e385fd/lib/lib_after.rs --- 1/2 --- Rust
24 #![cfg_attr(any(unix, target_os = "redox"), feature(libc))]                                                                                               24 #![cfg_attr(any(unix, target_os = "cloudabi", target_os = "redox"), feature(libc))]

File_Code/rust/c661e385fd/lib/lib_after.rs --- 2/2 --- Rust
119 #[cfg(any(unix, target_os = "redox"))]                                                                                                                   119 #[cfg(any(unix, target_os = "cloudabi", target_os = "redox"))]

