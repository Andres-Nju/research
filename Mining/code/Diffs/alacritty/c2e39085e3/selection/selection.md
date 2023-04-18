File_Code/alacritty/c2e39085e3/selection/selection_after.rs --- Rust
                                                                                                                                                           350             // Wrap to next line when selection starts to the right of last column
                                                                                                                                                           351             if start.point.col == num_cols {
                                                                                                                                                           352                 start.point = Point::new(start.point.line.saturating_sub(1), Column(0));
                                                                                                                                                           353             }

