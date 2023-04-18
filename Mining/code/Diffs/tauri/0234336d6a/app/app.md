File_Code/tauri/0234336d6a/app/app_after.rs --- Rust
265     (self.setup)(&mut app)?;                                                                                                                             265     (self.setup)(&mut app).map_err(|e| crate::Error::Setup(e.to_string()))?;

