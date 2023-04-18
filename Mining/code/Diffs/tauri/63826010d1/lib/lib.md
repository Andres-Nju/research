File_Code/tauri/63826010d1/lib/lib_after.rs --- Rust
18       Err(Error::new(Status::GenericFailure, e.to_string())),                                                                                             18       Err(Error::new(Status::GenericFailure, format!("{:#}", e))),

