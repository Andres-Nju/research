File_Code/cargo/2a9eacb1d0/workspaces/workspaces_after.rs --- Rust
1190         .file("ws/Cargo.toml", r#"                                                                                                                      1190         .file("ws/Cargo.toml", r#"
1191             [project]                                                                                                                                   1191             [project]
1192             name = "ws"                                                                                                                                 1192             name = "ws"
1193             version = "0.1.0"                                                                                                                           1193             version = "0.1.0"
1194             authors = []                                                                                                                                1194             authors = []
1195                                                                                                                                                         1195 
1196             [dependencies]                                                                                                                              1196             [dependencies]
1197             foo = { path = "../foo" }                                                                                                                   1197             foo = { path = "../foo" }
1198                                                                                                                                                         1198 
....                                                                                                                                                         1199             [workspace]
1199         "#)                                                                                                                                             1200         "#)

