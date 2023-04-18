File_Code/rust/394c23b084/error/error_after.rs --- Rust
                                                                                                                                                           215 #[stable(feature = "fmt_error", since = "1.11.0")]
                                                                                                                                                           216 impl Error for fmt::Error {
                                                                                                                                                           217     fn description(&self) -> &str {
                                                                                                                                                           218         "an error occurred when formatting an argument"
                                                                                                                                                           219     }
                                                                                                                                                           220 }

