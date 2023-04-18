File_Code/rust-analyzer/0fd87cbc47/char/char_after.rs --- Rust
 92     if !text.is_ascii() {                                                                                                                                 . 
 93         // TODO: Give a more precise error message (say what the invalid character was)                                                                   . 
 94         errors.push(SyntaxError::new(AsciiCodeEscapeOutOfRange, range));                                                                                  . 
 95     }                                                                                                                                                     . 
 96     if text.len() < 4 {                                                                                                                                  92     if !text.is_ascii() {
 ..                                                                                                                                                          93         // TODO: Give a more precise error message (say what the invalid character was)
 ..                                                                                                                                                          94         errors.push(SyntaxError::new(AsciiCodeEscapeOutOfRange, range));
 ..                                                                                                                                                          95     } else if text.chars().count() < 4 {
 97         errors.push(SyntaxError::new(TooShortAsciiCodeEscape, range));                                                                                   96         errors.push(SyntaxError::new(TooShortAsciiCodeEscape, range));
 98     } else {                                                                                                                                             97     } else {
 99         assert_eq!(                                                                                                                                      98         assert_eq!(
100             text.len(),                                                                                                                                  99             text.chars().count(),

