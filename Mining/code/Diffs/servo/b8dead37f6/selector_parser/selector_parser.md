File_Code/servo/b8dead37f6/selector_parser/selector_parser_after.rs --- Rust
                                                                                                                                                           344                         // Selectors inside `:-moz-any` may not include combinators.
                                                                                                                                                           345                         if selectors.iter().any(|s| s.next.is_some()) {
                                                                                                                                                           346                             return Err(())
                                                                                                                                                           347                         }

