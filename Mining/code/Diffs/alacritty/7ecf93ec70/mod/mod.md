File_Code/alacritty/7ecf93ec70/mod/mod_after.rs --- Rust
526                 self.display_offset = min(self.display_offset + *positions, self.len() - num_lines);                                                     526                 self.display_offset = min(self.display_offset + *positions, self.max_scroll_limit);

