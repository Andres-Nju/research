File_Code/alacritty/bb7b1201c9/mod/mod_after.rs --- Rust
   .                                                                                                                                                         1064         let last_visible_line = terminal.screen_lines() - 1;
1064         for hint in self.highlighted_hint.iter().chain(&self.vi_highlighted_hint) {                                                                     1065         for hint in self.highlighted_hint.iter().chain(&self.vi_highlighted_hint) {
1065             for point in (hint.bounds.start().line.0..=hint.bounds.end().line.0).flat_map(|line| {                                                      1066             for point in (hint.bounds.start().line.0..=hint.bounds.end().line.0).flat_map(|line| {
1066                 term::point_to_viewport(display_offset, Point::new(Line(line), Column(0)))                                                              1067                 term::point_to_viewport(display_offset, Point::new(Line(line), Column(0)))
                                                                                                                                                             1068                     .filter(|point| point.line <= last_visible_line)

