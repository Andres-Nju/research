File_Code/alacritty/860adde457/main/main_after.rs --- Rust
                                                                                                                                                            11 #[cfg(not(any(feature = "x11", feature = "wayland", target_os = "macos", windows)))]
                                                                                                                                                            12 compile_error!(r#"at least one of the "x11"/"wayland" features must be enabled"#);

