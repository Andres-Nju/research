File_Code/rust/9bf5773114/process/process_after.rs --- 1/2 --- Rust
938     ///     let mut stdin = child.stdin.as_mut().expect("Failed to open stdin");                                                                         938     ///     let stdin = child.stdin.as_mut().expect("Failed to open stdin");

File_Code/rust/9bf5773114/process/process_after.rs --- 2/2 --- Rust
943     /// assert_eq!(String::from_utf8_lossy(&output.stdout), "!dlrow ,olleH\n");                                                                          943     /// assert_eq!(String::from_utf8_lossy(&output.stdout), "!dlrow ,olleH");

