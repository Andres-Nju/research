File_Code/alacritty/5ac42bb13b/display/display_after.rs --- Rust
250         self.size_info.cell_width = (metrics.average_advance + config.font().offset().x as f64) as f32;                                                  250         self.size_info.cell_width = ((metrics.average_advance + config.font().offset().x as f64) as f32).floor();
251         self.size_info.cell_height = (metrics.line_height + config.font().offset().y as f64) as f32;                                                     251         self.size_info.cell_height = ((metrics.line_height + config.font().offset().y as f64) as f32).floor();

