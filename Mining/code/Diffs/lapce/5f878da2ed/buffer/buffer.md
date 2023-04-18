File_Code/lapce/5f878da2ed/buffer/buffer_after.rs --- Rust
57         let path = self.path.canonicalize()?;                                                                                                             57         let path = if self.path.is_symlink() {
                                                                                                                                                             58             self.path.canonicalize()?
                                                                                                                                                             59         } else {
                                                                                                                                                             60             self.path.clone()
                                                                                                                                                             61         };

