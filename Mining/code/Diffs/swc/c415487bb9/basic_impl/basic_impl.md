File_Code/swc/c415487bb9/basic_impl/basic_impl_after.rs --- Rust
                                                                                                                                                           187         {
                                                                                                                                                           188             let line_start_of_s = compute_line_starts(s);
                                                                                                                                                           189             if line_start_of_s.len() > 1 {
                                                                                                                                                           190                 self.line_count = self.line_count + line_start_of_s.len() - 1;
                                                                                                                                                           191                 self.line_pos = s.len() - line_start_of_s.last().cloned().unwrap_or(0);
                                                                                                                                                           192             }
                                                                                                                                                           193         }

