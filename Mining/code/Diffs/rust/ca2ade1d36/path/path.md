File_Code/rust/ca2ade1d36/path/path_after.rs --- Rust
   .                                                                                                                                                         1498         // FIXME: Remove target_os = "redox" and allow Redox prefixes
1498         self.has_root() && (cfg!(unix) || self.prefix().is_some())                                                                                      1499         self.has_root() && (cfg!(unix) || cfg!(target_os = "redox") || self.prefix().is_some())

