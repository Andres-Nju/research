File_Code/alacritty/3a9b8e65dd/conpty/conpty_after.rs --- Rust
237             cwd.map_or_else(ptr::null, |s| s.as_ptr()),                                                                                                  237             cwd.as_ref().map_or_else(ptr::null, |s| s.as_ptr()),

