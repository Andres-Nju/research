File_Code/nushell/90fae903ce/filesystem_shell/filesystem_shell_after.rs --- 1/3 --- Rust
  .                                                                                                                                                          674                     #[cfg(unix)]
  .                                                                                                                                                          675                     let is_fifo = metadata.file_type().is_fifo();
  .                                                                                                                                                          676 
674                     #[cfg(not(unix))]                                                                                                                    677                     #[cfg(not(unix))]
675                     let is_socket = false;                                                                                                               678                     let is_socket = false;
                                                                                                                                                             679                     #[cfg(not(unix))]
                                                                                                                                                             680                     let is_fifo = false;

File_Code/nushell/90fae903ce/filesystem_shell/filesystem_shell_after.rs --- 2/3 --- Rust
                                                                                                                                                             686                         || is_fifo

File_Code/nushell/90fae903ce/filesystem_shell/filesystem_shell_after.rs --- 3/3 --- Rust
702                             result = if metadata.is_file() || is_socket {                                                                                708                             result = if metadata.is_file() || is_socket || is_fifo {

