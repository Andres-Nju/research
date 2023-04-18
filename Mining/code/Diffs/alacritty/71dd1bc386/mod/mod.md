File_Code/alacritty/71dd1bc386/mod/mod_after.rs --- Rust
987                 res += &(self.line_to_string(line, start.col..end.col, start.col.0 != 0) + "\n");                                                        987                 res += &self.line_to_string(line, start.col..end.col, start.col.0 != 0);
                                                                                                                                                             988 
                                                                                                                                                             989                 // If the last column is included, newline is appended automatically
                                                                                                                                                             990                 if end.col != self.cols() - 1 {
                                                                                                                                                             991                     res += "\n";
                                                                                                                                                             992                 }

