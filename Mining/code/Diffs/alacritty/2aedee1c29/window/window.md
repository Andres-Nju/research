File_Code/alacritty/2aedee1c29/window/window_after.rs --- 1/3 --- Rust
327             let decoder = Decoder::new(Cursor::new(WINDOW_ICON));                                                                                        327             let mut decoder = Decoder::new(Cursor::new(WINDOW_ICON));
                                                                                                                                                             328             decoder.set_transformations(png::Transformations::normalize_to_color8());

File_Code/alacritty/2aedee1c29/window/window_after.rs --- 2/3 --- Rust
                                                                                                                                                             333                 .expect("invalid embedded icon format")

File_Code/alacritty/2aedee1c29/window/window_after.rs --- 3/3 --- Rust
344         let builder = builder.with_window_icon(icon.ok());                                                                                               346         let builder = builder.with_window_icon(Some(icon));

