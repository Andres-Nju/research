File_Code/rust-analyzer/4d66f5d7d2/name/name_after.rs --- Rust
96             ast::FieldKind::Index(idx) => Name::new_tuple_field(idx.text().parse().unwrap()),                                                             96             ast::FieldKind::Index(idx) => {
                                                                                                                                                             97                 let idx = idx.text().parse::<usize>().unwrap_or(0);
                                                                                                                                                             98                 Name::new_tuple_field(idx)
                                                                                                                                                             99             }

