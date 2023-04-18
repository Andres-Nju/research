File_Code/cargo/b9a4ef18c8/cargo_doc/cargo_doc_after.rs --- Rust
102     match Command::new("cmd").arg("/C").arg("start").arg("").arg(path).status() {                                                                        102     match Command::new("cmd").arg("/C").arg(path).status() {
103         Ok(_) => return Ok("cmd /C start"),                                                                                                              103         Ok(_) => return Ok("cmd /C"),
104         Err(_) => return Err(vec!["cmd /C start"])                                                                                                       104         Err(_) => return Err(vec!["cmd /C"])

