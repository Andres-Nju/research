File_Code/nushell/dd4351e2b7/filesystem_shell/filesystem_shell_after.rs --- Rust
157                     if e.kind() == ErrorKind::PermissionDenied {                                                                                         157                     if e.kind() == ErrorKind::PermissionDenied || e.kind() == ErrorKind::Other {

