File_Code/alacritty/1e064fb3e7/input/input_after.rs --- Rust
227             let cell_x = x as usize % size_info.cell_width as usize;                                                                                     227             let cell_x = (x as usize - size_info.padding_x as usize) % size_info.cell_width as usize;

