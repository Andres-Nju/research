File_Code/rust/de59d5d737/main/main_after.rs --- Rust
181                         toml::encode(&manifest_version));                                                                                                181                         toml::Value::String(manifest_version));
182         manifest.insert("date".to_string(), toml::encode(&date));                                                                                        182         manifest.insert("date".to_string(), toml::Value::String(date));

