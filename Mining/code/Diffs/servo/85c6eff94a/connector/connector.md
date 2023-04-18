File_Code/servo/85c6eff94a/connector/connector_after.rs --- 1/3 --- Rust
84                             decoder.get_mut().get_mut().extend(&chunk.into_bytes());                                                                      84                             decoder.get_mut().get_mut().extend(chunk.as_ref());

File_Code/servo/85c6eff94a/connector/connector_after.rs --- 2/3 --- Rust
99                             decoder.get_mut().get_mut().extend(&chunk.into_bytes());                                                                      99                             decoder.get_mut().get_mut().extend(chunk.as_ref());

File_Code/servo/85c6eff94a/connector/connector_after.rs --- 3/3 --- Rust
106                             decoder.get_mut().get_mut().extend(&chunk.into_bytes());                                                                     106                             decoder.get_mut().get_mut().extend(chunk.as_ref());

