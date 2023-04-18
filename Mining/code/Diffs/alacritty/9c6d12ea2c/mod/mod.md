File_Code/alacritty/9c6d12ea2c/mod/mod_after.rs --- Rust
1417                 let col = self.cursor.point.col.0.saturating_sub(1);                                                                                    1417                 let mut col = self.cursor.point.col.0.saturating_sub(1);
1418                 let line = self.cursor.point.line;                                                                                                      1418                 let line = self.cursor.point.line;
1419                 if self.grid[line][Column(col)].flags.contains(cell::Flags::WIDE_CHAR_SPACER) {                                                         1419                 if self.grid[line][Column(col)].flags.contains(cell::Flags::WIDE_CHAR_SPACER) {
1420                     col.saturating_sub(1);                                                                                                              1420                     col = col.saturating_sub(1);

