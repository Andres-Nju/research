File_Code/alacritty/860adde457/build/build_after.rs --- Rust
1 #[cfg(not(any(feature = "x11", feature = "wayland", target_os = "macos", windows)))]                                                                         
2 compile_error!(r#"at least one of the "x11"/"wayland" features must be enabled"#);                                                                           

