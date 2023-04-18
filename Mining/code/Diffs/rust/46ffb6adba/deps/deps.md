File_Code/rust/46ffb6adba/deps/deps_after.rs --- Rust
244         *bad = *bad || !check_license(&toml);                                                                                                            244         *bad = !check_license(&toml) || *bad;

