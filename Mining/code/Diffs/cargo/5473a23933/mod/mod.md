File_Code/cargo/5473a23933/mod/mod_after.rs --- Rust
78                  handle_srderr: &mut FnMut(&str) -> CargoResult<()>)                                                                                      78                  handle_stderr: &mut FnMut(&str) -> CargoResult<()>)
79                  -> Result<(), ProcessError> {                                                                                                            79                  -> Result<(), ProcessError> {
80         cmd.exec_with_streaming(handle_stdout, handle_srderr)?;                                                                                           80         cmd.exec_with_streaming(handle_stdout, handle_stderr)?;

