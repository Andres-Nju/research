File_Code/alacritty/18a226fe45/mod/mod_after.rs --- 1/2 --- Rust
2086         self.mode = Default::default();                                                                                                                   

File_Code/alacritty/18a226fe45/mod/mod_after.rs --- 2/2 --- Rust
                                                                                                                                                             2098         // Preserve vi mode across resets.
                                                                                                                                                             2099         self.mode &= TermMode::VI;
                                                                                                                                                             2100         self.mode.insert(TermMode::default());

