File_Code/nushell/e4c512e33d/wrap/wrap_after.rs --- Rust
257                 current_line_width += 1 + subline.width;                                                                                                 257                 current_line_width += subline.width;
...                                                                                                                                                          258 
...                                                                                                                                                          259                 if current_line_width + 1 < cell_width {
...                                                                                                                                                          260                     current_line_width += 1;
258                 current_line.push(' ');                                                                                                                  261                     current_line.push(' ');
                                                                                                                                                             262                 }

