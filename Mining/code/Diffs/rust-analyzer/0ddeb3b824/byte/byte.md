File_Code/rust-analyzer/0ddeb3b824/byte/byte_after.rs --- Rust
91     if text.len() < 4 {                                                                                                                                   91     if !text.is_ascii() {
                                                                                                                                                             92         errors.push(SyntaxError::new(MalformedByteCodeEscape, range));
                                                                                                                                                             93     } else if text.chars().count() < 4 {

