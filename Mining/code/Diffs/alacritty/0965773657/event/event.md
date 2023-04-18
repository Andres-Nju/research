File_Code/alacritty/0965773657/event/event_after.rs --- 1/2 --- Rust
                                                                                                                                                           856         self.terminal.vi_mode_cursor.point = self.search_state.origin;

File_Code/alacritty/0965773657/event/event_after.rs --- 2/2 --- Rust
858         self.terminal.vi_mode_cursor.point =                                                                                                                 
859             self.search_state.origin.grid_clamp(self.terminal, Boundary::Grid);                                                                              

