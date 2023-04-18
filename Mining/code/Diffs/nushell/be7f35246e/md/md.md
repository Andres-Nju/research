File_Code/nushell/be7f35246e/md/md_after.rs --- 1/2 --- Rust
                                                                                                                                                            57             Example {
                                                                                                                                                            58                 description: "Render a list",
                                                                                                                                                            59                 example: "[0 1 2] | to md --pretty",
                                                                                                                                                            60                 result: Some(Value::test_string("0\n1\n2")),
                                                                                                                                                            61             },

File_Code/nushell/be7f35246e/md/md_after.rs --- 2/2 --- Rust
294             if pretty {                                                                                                                                  299             if pretty && column_widths.get(i).is_some() {

