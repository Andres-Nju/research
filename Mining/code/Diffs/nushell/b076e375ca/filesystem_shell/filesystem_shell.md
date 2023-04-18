File_Code/nushell/b076e375ca/filesystem_shell/filesystem_shell_after.rs --- 1/3 --- Rust
                                                                                                                                                           599                     #[cfg(unix)]
                                                                                                                                                           600                     let is_socket = metadata.file_type().is_socket();
                                                                                                                                                           601                     #[cfg(not(unix))]
                                                                                                                                                           602                     let is_socket = false;

File_Code/nushell/b076e375ca/filesystem_shell/filesystem_shell_after.rs --- 2/3 --- Rust
                                                                                                                                                           607                         || is_socket

File_Code/nushell/b076e375ca/filesystem_shell/filesystem_shell_after.rs --- 3/3 --- Rust
623                             result = if metadata.is_file() {                                                                                             629                             result = if metadata.is_file() || is_socket {

