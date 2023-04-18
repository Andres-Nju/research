File_Code/alacritty/96ea5c445e/search/search_after.rs --- Rust
423         while self.grid[point.line][self.cols() - 1].flags.contains(Flags::WRAPLINE) {                                                                   423         while point.line > 0
                                                                                                                                                             424             && self.grid[point.line][self.cols() - 1].flags.contains(Flags::WRAPLINE)

