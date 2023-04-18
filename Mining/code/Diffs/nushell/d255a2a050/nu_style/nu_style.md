File_Code/nushell/d255a2a050/nu_style/nu_style_after.rs --- 1/2 --- Rust
14         Some(fg) => color_from_hex(&fg).expect("error with foreground color"),                                                                            14         Some(fg) => color_from_hex(&fg).unwrap_or_default(),

File_Code/nushell/d255a2a050/nu_style/nu_style_after.rs --- 2/2 --- Rust
19         Some(bg) => color_from_hex(&bg).expect("error with background color"),                                                                            19         Some(bg) => color_from_hex(&bg).unwrap_or_default(),

