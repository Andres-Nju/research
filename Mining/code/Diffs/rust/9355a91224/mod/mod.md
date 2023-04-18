File_Code/rust/9355a91224/mod/mod_after.rs --- 1/3 --- Rust
41 #[derive(Clone)]                                                                                                                                          41 #[derive(Clone, Debug)]

File_Code/rust/9355a91224/mod/mod_after.rs --- 2/3 --- Rust
47 #[derive(Clone, PartialOrd, Ord, PartialEq, Eq)]                                                                                                          47 #[derive(Clone, Debug, PartialOrd, Ord, PartialEq, Eq)]

File_Code/rust/9355a91224/mod/mod_after.rs --- 3/3 --- Rust
                                                                                                                                                            495                             assert!(rendered_lines.len() >= 2,
                                                                                                                                                            496                                     "no annotations resulted from: {:?}",
                                                                                                                                                            497                                     line);

