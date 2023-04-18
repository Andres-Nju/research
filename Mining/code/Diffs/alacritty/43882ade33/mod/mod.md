File_Code/alacritty/43882ade33/mod/mod_after.rs --- Rust
1212     fn scroll_down_relative(&mut self, origin: Line, lines: Line) {                                                                                     1212     fn scroll_down_relative(&mut self, origin: Line, mut lines: Line) {
1213         trace!("scroll_down_relative: origin={}, lines={}", origin, lines);                                                                             1213         trace!("scroll_down_relative: origin={}, lines={}", origin, lines);
1214         let lines = min(lines, self.scroll_region.end - self.scroll_region.start);                                                                      1214         lines = min(lines, self.scroll_region.end - self.scroll_region.start);
                                                                                                                                                             1215         lines = min(lines, self.scroll_region.end - origin);

