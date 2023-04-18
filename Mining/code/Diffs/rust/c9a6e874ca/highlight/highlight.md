File_Code/rust/c9a6e874ca/highlight/highlight_after.rs --- Rust
221             // If this '&' token is directly adjacent to another token, assume                                                                           221             // If this '&' or '*' token is followed by a non-whitespace token, assume that it's the
222             // that it's the address-of operator instead of the and-operator.                                                                            222             // reference or dereference operator or a reference or pointer type, instead of the
...                                                                                                                                                          223             // bit-and or multiplication operator.
223             token::BinOp(token::And) if self.lexer.peek().sp.lo == tas.sp.hi => Class::RefKeyWord,                                                       224             token::BinOp(token::And) | token::BinOp(token::Star)
                                                                                                                                                             225                 if self.lexer.peek().tok != token::Whitespace => Class::RefKeyWord,

