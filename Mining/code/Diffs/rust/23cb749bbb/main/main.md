File_Code/rust/23cb749bbb/main/main_after.rs --- Rust
118                 } else {                                                                                                                                 118                 } else if !err.is_http() {
                                                                                                                                                             119                     eprintln!("Non-HTTP-related error for link: {} {}", link.link.uri, err);
                                                                                                                                                             120                 } else {

