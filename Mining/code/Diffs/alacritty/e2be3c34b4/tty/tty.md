File_Code/alacritty/e2be3c34b4/tty/tty_after.rs --- 1/2 --- Rust
                                                                                                                                                           196     // Ownership of fd is transferred to the Stdio structs and will be closed by them at the end of
                                                                                                                                                           197     // this scope. (It is not an issue that the fd is closed three times since File::drop ignores
                                                                                                                                                           198     // error on libc::close.)

File_Code/alacritty/e2be3c34b4/tty/tty_after.rs --- 2/2 --- Rust
252                 // Parent doesn't need slave fd                                                                                                              
253                 libc::close(slave);                                                                                                                          

