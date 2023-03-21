Codes/tauri/7de67f6dd9/wix/build_wix_app_installer_after.rs --- 1/2 --- Rust
 9     "x86_64-pc-windows-msvc" => "amd64",                                                                                                                   9     "x86_64-pc-windows-msvc" => "x64",
10     target => return Err(format!("unsupported target: {}", target)),                                                                                      10     target => return Err(format!("unsupported target: {}", target)),
11   };                                                                                                                                                      11   };
                                                                                                                                                             12 
                                                                                                                                                             13   info!(logger, "Target: {}", settings.binary_arch());

Codes/tauri/7de67f6dd9/wix/build_wix_app_installer_after.rs --- 2/2 --- Rust
32     panic!("unsupported target: {}");                                                                                                                     34     return Err(format!("unsupported target: {}", arch));

