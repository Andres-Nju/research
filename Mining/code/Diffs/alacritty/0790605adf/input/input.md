File_Code/alacritty/0790605adf/input/input_after.rs --- 1/2 --- Rust
827             ElementState::Released => (),                                                                                                                827             ElementState::Released => *self.ctx.suppress_chars() = false,

File_Code/alacritty/0790605adf/input/input_after.rs --- 2/2 --- Rust
857             *self.ctx.suppress_chars() = false;                                                                                                              

