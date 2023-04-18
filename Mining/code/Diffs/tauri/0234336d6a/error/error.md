File_Code/tauri/0234336d6a/error/error_after.rs --- Rust
54   #[error("error encountered during setup hood: {0}")]                                                                                                    54   #[error("error encountered during setup hook: {0}")]
55   Setup(#[from] Box<dyn std::error::Error>),                                                                                                              55   Setup(String),

