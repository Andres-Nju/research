File_Code/rust/f94c075ab4/diagnostics/diagnostics_after.rs --- Rust
4548 E0648: r##"                                                                                                                                             4548 E0648: r##"
4549 `export_name` attributes may not contain null characters (`\0`).                                                                                        4549 `export_name` attributes may not contain null characters (`\0`).
4550                                                                                                                                                         4550 
4551 ```compile_fail,E0648                                                                                                                                   4551 ```compile_fail,E0648
4552 #[export_name="\0foo"] // error: `export_name` may not contain null characters                                                                          4552 #[export_name="\0foo"] // error: `export_name` may not contain null characters
4553 ```                                                                                                                                                     4553 pub fn bar() {}
....                                                                                                                                                         4554 ```
4554 "##,                                                                                                                                                    4555 "##,

