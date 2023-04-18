File_Code/rust/322e46916e/mod/mod_after.rs --- Rust
243                            override_span: Option<Span>) -> Result<Self, ()> {                                                                            243                            override_span: Option<Span>,
...                                                                                                                                                          244                            prepend_error_text: &str) -> Result<Self, ()> {
244         let mut sr = StringReader::new_raw(sess, source_file, override_span);                                                                            245         let mut sr = StringReader::new_raw(sess, source_file, override_span);
245         if sr.advance_token().is_err() {                                                                                                                 246         if sr.advance_token().is_err() {
                                                                                                                                                             247             eprintln!("{}", prepend_error_text);

