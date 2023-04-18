File_Code/alacritty/82c9235bb1/mod/mod_after.rs --- Rust
                                                                                                                                                          1089         self.cursor.point.line = min(self.cursor.point.line, self.grid.num_lines() - 1);
                                                                                                                                                          1090         self.cursor.point.col = min(self.cursor.point.col, self.grid.num_cols() - 1);

