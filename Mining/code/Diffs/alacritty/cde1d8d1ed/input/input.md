File_Code/alacritty/cde1d8d1ed/input/input_after.rs --- Rust
  .                                                                                                                                                          307         let additional_padding = (size_info.width - size_info.padding_x * 2.) % size_info.cell_width;
  .                                                                                                                                                          308         let end_of_grid = size_info.width - size_info.padding_x - additional_padding;
307         let cell_side = if cell_x > half_cell_width                                                                                                      309         let cell_side = if cell_x > half_cell_width
308             // Edge case when mouse leaves the window                                                                                                    310             // Edge case when mouse leaves the window
309             || x as f32 >= size_info.width - size_info.padding_x                                                                                         311             || x as f32 >= end_of_grid

