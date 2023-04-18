File_Code/alacritty/e3af53c863/unix/unix_after.rs --- Rust
  .                                                                                                                                                          146     //
  .                                                                                                                                                          147     // XXX: we use zsh here over sh due to `exec -a`.
146     login_command.args(["-flp", pw.name, "/bin/sh", "-c", &exec]);                                                                                       148     login_command.args(["-flp", pw.name, "/bin/zsh", "-c", &exec]);

