File_Code/cargo/873f69c0b8/build-auth/build-auth_after.rs --- 1/3 --- Rust
169             "[[..]] failed to send request: [..]\n"                                                                                                      169             "[..]failed to send request: [..]"

File_Code/cargo/873f69c0b8/build-auth/build-auth_after.rs --- 2/3 --- Rust
176             "[..] SSL error: [..]"                                                                                                                       176             "[..]SSL error: [..]"

File_Code/cargo/873f69c0b8/build-auth/build-auth_after.rs --- 3/3 --- Rust
208                     .with_stderr_contains("\                                                                                                             208                     .with_stderr_contains("\
209 Caused by:                                                                                                                                               209 Caused by:
210   [[..]] failed to start SSH session: Failed getting banner                                                                                              210   [..]failed to start SSH session: Failed getting banner[..]
211 "));                                                                                                                                                     211 "));

