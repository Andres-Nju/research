File_Code/cargo/dd653ebe4c/shell/shell_after.rs --- 1/4 --- Rust
213         self.err.print(&"error:", Some(&message), Red, false)                                                                                            213         self.err.print(&"error", Some(&message), Red, false)

File_Code/cargo/dd653ebe4c/shell/shell_after.rs --- 2/4 --- Rust
220             _ => self.print(&"warning:", Some(&message), Yellow, false),                                                                                 220             _ => self.print(&"warning", Some(&message), Yellow, false),

File_Code/cargo/dd653ebe4c/shell/shell_after.rs --- 3/4 --- Rust
                                                                                                                                                             321                     stream.set_color(ColorSpec::new().set_bold(true))?;
                                                                                                                                                             322                     write!(stream, ":")?;

File_Code/cargo/dd653ebe4c/shell/shell_after.rs --- 4/4 --- Rust
332                     write!(w, "{}", status)?;                                                                                                            334                     write!(w, "{}:", status)?;

