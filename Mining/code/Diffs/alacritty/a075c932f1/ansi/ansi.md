File_Code/alacritty/a075c932f1/ansi/ansi_after.rs --- Rust
852                 if params.len() < 3 || params[1].is_empty() {                                                                                            852                 if params.len() < 3 {
853                     return unhandled(params);                                                                                                            853                     return unhandled(params);
854                 }                                                                                                                                        854                 }
855                                                                                                                                                          855 
...                                                                                                                                                          856                 let clipboard = params[1].get(0).unwrap_or(&b'c');
856                 match params[2] {                                                                                                                        857                 match params[2] {
857                     b"?" => self.handler.write_clipboard(params[1][0], writer),                                                                          858                     b"?" => self.handler.write_clipboard(*clipboard, writer),
858                     base64 => self.handler.set_clipboard(params[1][0], base64),                                                                          859                     base64 => self.handler.set_clipboard(*clipboard, base64),

