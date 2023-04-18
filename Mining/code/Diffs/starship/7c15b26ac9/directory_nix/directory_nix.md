File_Code/starship/7c15b26ac9/directory_nix/directory_nix_after.rs --- 1/2 --- Rust
35 #[cfg(all(unix, not(target_os = "macos")))]                                                                                                               35 #[cfg(all(unix, not(any(target_os = "macos", target_os = "ios"))))]

File_Code/starship/7c15b26ac9/directory_nix/directory_nix_after.rs --- 2/2 --- Rust
43 #[cfg(all(unix, target_os = "macos"))]                                                                                                                    43 #[cfg(all(unix, any(target_os = "macos", target_os = "ios")))]

