File_Code/rust-analyzer/e60ef6260f/syntax_ptr/syntax_ptr_after.rs --- 1/2 --- Rust
26 /// without retainig syntax tree in memory. You need to explicitelly `resovle`                                                                            26 /// without retaining syntax tree in memory. You need to explicitly `resolve`

File_Code/rust-analyzer/e60ef6260f/syntax_ptr/syntax_ptr_after.rs --- 2/2 --- Rust
83                 .unwrap_or_else(|| panic!("can't resovle local ptr to SyntaxNode: {:?}", self))                                                           83                 .unwrap_or_else(|| panic!("can't resolve local ptr to SyntaxNode: {:?}", self))

