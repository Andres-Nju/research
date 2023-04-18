File_Code/alacritty/875167a510/mod/mod_after.rs --- 1/4 --- Rust
                                                                                                                                                           686         if self.alt {
                                                                                                                                                           687             let template = self.empty_cell;
                                                                                                                                                           688             self.grid.clear(|c| c.reset(&template));
                                                                                                                                                           689         }

File_Code/alacritty/875167a510/mod/mod_after.rs --- 2/4 --- Rust
689         if self.alt {                                                                                                                                        
690             let template = self.empty_cell;                                                                                                                  
691             self.grid.clear(|c| c.reset(&template));                                                                                                         
692         }                                                                                                                                                    

File_Code/alacritty/875167a510/mod/mod_after.rs --- 3/4 --- Rust
                                                                                                                                                            1194                 self.save_cursor_position();

File_Code/alacritty/875167a510/mod/mod_after.rs --- 4/4 --- Rust
                                                                                                                                                            1216                 self.restore_cursor_position();

