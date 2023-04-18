File_Code/nushell/e514204db0/lib/lib_after.rs --- Rust
18 pub const NATIVE_PATH_ENV_SEPARATOR: char = ':';                                                                                                          18 pub const NATIVE_PATH_ENV_SEPARATOR: char = ';';
19 #[cfg(not(windows))]                                                                                                                                      19 #[cfg(not(windows))]
20 pub const NATIVE_PATH_ENV_SEPARATOR: char = ';';                                                                                                          20 pub const NATIVE_PATH_ENV_SEPARATOR: char = ':';

