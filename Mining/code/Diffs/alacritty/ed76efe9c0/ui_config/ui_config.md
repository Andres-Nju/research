File_Code/alacritty/ed76efe9c0/ui_config/ui_config_after.rs --- 1/3 --- Rust
73     window_opacity: Option<Percentage>,                                                                                                                   73     background_opacity: Option<Percentage>,

File_Code/alacritty/ed76efe9c0/ui_config/ui_config_after.rs --- 2/3 --- Rust
88             window_opacity: Default::default(),                                                                                                           88             background_opacity: Default::default(),

File_Code/alacritty/ed76efe9c0/ui_config/ui_config_after.rs --- 3/3 --- Rust
120         self.window_opacity.unwrap_or(self.window.opacity).as_f32()                                                                                      120         self.background_opacity.unwrap_or(self.window.opacity).as_f32()

