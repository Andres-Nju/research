File_Code/rust/6aafdc3781/parser/parser_after.rs --- Text (6 errors, exceeded DFT_PARSE_ERROR_LIMIT)
3436                 let mut err = self.struct_span_err(self.span, "unexpected token `||` after pattern");                                                   3436                 let mut err = self.struct_span_err(self.span,
....                                                                                                                                                         3437                                                    "unexpected token `||` after pattern");
3437                 err.span_suggestion(self.span, "use a single `|` to specify multiple patterns", "|".to_owned());                                        3438                 err.span_suggestion(self.span,
                                                                                                                                                             3439                                     "use a single `|` to specify multiple patterns",
                                                                                                                                                             3440                                     "|".to_owned());

