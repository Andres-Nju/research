File_Code/nushell/eb875ea949/glob/glob_after.rs --- 1/3 --- Rust
87         r#"For more glob pattern help please refer to https://github.com/olson-sean-k/wax"#                                                               87         r#"For more glob pattern help, please refer to https://github.com/olson-sean-k/wax"#

File_Code/nushell/eb875ea949/glob/glob_after.rs --- 2/3 --- Rust
105                 "".to_string(),                                                                                                                          105                 "glob pattern is empty".to_string(),

File_Code/nushell/eb875ea949/glob/glob_after.rs --- 3/3 --- Rust
123                     "".to_string(),                                                                                                                      ... 
124                     None,                                                                                                                                ... 
125                     Some(format!("{}", e)),                                                                                                              123                     format!("{}", e),
                                                                                                                                                             124                     Some(glob_pattern.span),
                                                                                                                                                             125                     None,

