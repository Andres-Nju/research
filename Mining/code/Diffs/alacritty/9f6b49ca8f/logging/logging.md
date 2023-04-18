File_Code/alacritty/9f6b49ca8f/logging/logging_after.rs --- Rust
187             let file = OpenOptions::new().append(true).create(true).open(&self.path);                                                                    187             let file = OpenOptions::new().append(true).create_new(true).open(&self.path);

