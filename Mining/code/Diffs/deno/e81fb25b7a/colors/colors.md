File_Code/deno/e81fb25b7a/colors/colors_after.rs --- 1/2 --- Rust
                                                                                                                                                            38   if !use_color() {
                                                                                                                                                            39     return String::from(s);
                                                                                                                                                            40   }

File_Code/deno/e81fb25b7a/colors/colors_after.rs --- 2/2 --- Rust
40   if use_color() {                                                                                                                                        .. 
41     ansi_writer.set_color(&colorspec).unwrap();                                                                                                           43   ansi_writer.set_color(&colorspec).unwrap();
42   }                                                                                                                                                          

