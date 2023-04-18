File_Code/cargo/fb6a98fbd6/flock/flock_after.rs --- Rust
                                                                                                                                                           279         #[cfg(target_os = "linux")]
                                                                                                                                                           280         Err(ref e) if e.raw_os_error() == Some(libc::ENOSYS) => return Ok(()),

