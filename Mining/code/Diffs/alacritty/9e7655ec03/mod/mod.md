File_Code/alacritty/9e7655ec03/mod/mod_after.rs --- Rust
  .                                                                                                                                                          515         let viewport_bottom = Line(-(self.grid.display_offset() as i32));
  .                                                                                                                                                          516         let viewport_top = viewport_bottom + self.bottommost_line();
515         self.vi_mode_cursor.point.column = min(vi_point.column, Column(num_cols - 1));                                                                   517         self.vi_mode_cursor.point.line = max(min(vi_point.line, viewport_top), viewport_bottom);
516         self.vi_mode_cursor.point.line = min(vi_point.line, Line(num_lines as i32 - 1));                                                                 518         self.vi_mode_cursor.point.column = min(vi_point.column, self.last_column());

