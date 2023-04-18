File_Code/alacritty/72088dafec/mod/mod_after.rs --- Rust
  .                                                                                                                                                          439                         // Using `self.cursor.line` leads to inconsitent cursor position when
  .                                                                                                                                                          440                         // scrolling. See https://github.com/jwilm/alacritty/issues/2570 for more
  .                                                                                                                                                          441                         // info.
439                         line: self.cursor.line,                                                                                                          442                         line: self.inner.line(),

