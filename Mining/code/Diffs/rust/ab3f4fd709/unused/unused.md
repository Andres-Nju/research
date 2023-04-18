File_Code/rust/ab3f4fd709/unused/unused_after.rs --- 1/2 --- Rust
20 use syntax_pos::{MultiSpan, Span, BytePos};                                                                                                               20 use syntax_pos::{Span, BytePos};

File_Code/rust/ab3f4fd709/unused/unused_after.rs --- 2/2 --- Rust
359                     !MultiSpan::from(value.span).primary_span()                                                                                          359                     !value.span.from_expansion()
360                         .map_or(false, |span| span.from_expansion())                                                                                         

