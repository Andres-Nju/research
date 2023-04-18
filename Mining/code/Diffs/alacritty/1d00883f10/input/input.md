File_Code/alacritty/1d00883f10/input/input_after.rs --- 1/3 --- Rust
866             ElementState::Released => *self.ctx.suppress_chars() = false,                                                                                866             ElementState::Released => (),

File_Code/alacritty/1d00883f10/input/input_after.rs --- 2/3 --- Rust
                                                                                                                                                             895             *self.ctx.suppress_chars() = false;

File_Code/alacritty/1d00883f10/input/input_after.rs --- 3/3 --- Rust
954                 // Don't suppress when there has been a `ReceiveChar` action.                                                                            956                 // Pass through the key if any of the bindings has the `ReceiveChar` action.

