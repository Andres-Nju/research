File_Code/nushell/774be79321/parse/parse_after.rs --- Rust
1757         let name = lite_cmd.parts[0]                                                                                                                    1757         let mut name = lite_cmd.parts[0]
1758             .clone()                                                                                                                                    1758             .clone()
1759             .map(|v| v.chars().skip(1).collect::<String>());                                                                                            1759             .map(|v| v.chars().skip(1).collect::<String>());
                                                                                                                                                             1760 
                                                                                                                                                             1761         name.span = Span::new(name.span.start() + 1, name.span.end());

