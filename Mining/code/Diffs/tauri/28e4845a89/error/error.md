File_Code/tauri/28e4845a89/error/error_after.rs --- Rust
61   #[error("invalid args for command `{0}`: {1}")]                                                                                                         61   #[error("invalid args `{1}` for command `{0}`: {2}")]
62   InvalidArgs(&'static str, serde_json::Error),                                                                                                           62   InvalidArgs(&'static str, &'static str, serde_json::Error),

