File_Code/alacritty/62e9d3ab39/builtin_font/builtin_font_after.rs --- Rust
 .                                                                                                                                                           40     // Ensure that width and height is at least one.
40     let height = (metrics.line_height as i32 + offset.y as i32) as usize;                                                                                 41     let height = (metrics.line_height as i32 + offset.y as i32).max(1) as usize;
41     let width = (metrics.average_advance as i32 + offset.x as i32) as usize;                                                                              42     let width = (metrics.average_advance as i32 + offset.x as i32).max(1) as usize;

