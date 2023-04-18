File_Code/nushell/c2f8f4bd9b/int/int_after.rs --- 1/2 --- Rust
294         _ => match a_string.parse::<i64>() {                                                                                                             294         _ => match trimmed.parse::<i64>() {

File_Code/nushell/c2f8f4bd9b/int/int_after.rs --- 2/2 --- Rust
...                                                                                                                                                          302                     Some(format!(
...                                                                                                                                                          303                         r#"string "{}" does not represent a valid integer"#,
...                                                                                                                                                          304                         trimmed
302                     None,                                                                                                                                305                     )),

