File_Code/wezterm/318267c88b/terminfo/terminfo_after.rs --- 1/2 --- Rust
453                     self.cursor_up(*n as u32, out)?;                                                                                                     453                     self.cursor_up(-*n as u32, out)?;

File_Code/wezterm/318267c88b/terminfo/terminfo_after.rs --- 2/2 --- Rust
459                     self.cursor_left(*n as u32, out)?;                                                                                                   459                     self.cursor_left(-*n as u32, out)?;

