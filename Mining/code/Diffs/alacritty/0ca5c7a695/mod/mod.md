File_Code/alacritty/0ca5c7a695/mod/mod_after.rs --- Rust
1141         self.grid.resize(num_lines, num_cols, &self.cursor.template);                                                                                   1141         self.grid.resize(num_lines, num_cols, &Cell::default());
1142         self.alt_grid.resize(num_lines, num_cols, &self.cursor_save_alt.template);                                                                      1142         self.alt_grid.resize(num_lines, num_cols, &Cell::default());

