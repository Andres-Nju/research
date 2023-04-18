File_Code/alacritty/78795522e9/mod/mod_after.rs --- Rust
795             if wide && point.column + 1 < self.columns() {                                                                                               795             if wide && point.column <= self.last_column() {
796                 self.grid[point.line][point.column + 1].flags.remove(Flags::WIDE_CHAR_SPACER);                                                           796                 self.grid[point.line][point.column + 1].flags.remove(Flags::WIDE_CHAR_SPACER);
797             } else {                                                                                                                                     797             } else if point.column > 0 {

