File_Code/alacritty/8cda3d1405/event/event_after.rs --- Rust
                                                                                                                                                           424             // Adjust origin for content moving upward on search start.
                                                                                                                                                           425             if self.terminal.grid().cursor.point.line + 1 == self.terminal.screen_lines() {
                                                                                                                                                           426                 self.search_state.origin.line -= 1;
                                                                                                                                                           427             }

