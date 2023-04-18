File_Code/servo/e136863939/selector_parser/selector_parser_after.rs --- Rust
  .                                                                                                                                                          238                             // FIXME(emilio): Avoid the extra allocation!
238                             let mut css = CssStringWriter::new(dest);                                                                                    239                             let mut css = CssStringWriter::new(dest);
...                                                                                                                                                          240 
...                                                                                                                                                          241                             // Discount the null char in the end from the
...                                                                                                                                                          242                             // string.
239                             css.write_str(&String::from_utf16(&s).unwrap())?;                                                                            243                             css.write_str(&String::from_utf16(&s[..s.len() - 1]).unwrap())?;

