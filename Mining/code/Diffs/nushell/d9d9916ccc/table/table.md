File_Code/nushell/d9d9916ccc/table/table_after.rs --- Rust
1619             Ok(Some(table)) => Some(Ok(table.as_bytes().to_vec())),                                                                                     1619             Ok(Some(table)) => {
                                                                                                                                                             1620                 let mut bytes = table.as_bytes().to_vec();
                                                                                                                                                             1621                 bytes.push(b'\n'); // tabled tables don't come with a newline on the end
                                                                                                                                                             1622                 Some(Ok(bytes))
                                                                                                                                                             1623             }

