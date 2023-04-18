File_Code/servo/a5ce6304b1/build/build_after.rs --- Rust
 .                                                                                                                                                           40         let link = std::process::Command::new("where").arg("link.exe").output().unwrap();
 .                                                                                                                                                           41         let link_path: Vec<&str> = std::str::from_utf8(&link.stdout).unwrap().split("\r\n").collect();
40         build.define("CMAKE_LINKER", "C:\\Program Files (x86)\\Microsoft Visual Studio 14.0\\VC\\bin\\amd64\\link.exe");                                  42         build.define("CMAKE_LINKER", link_path[0]);

