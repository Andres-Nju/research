File_Code/rust/22c4ee365c/parser/parser_after.rs --- Text (4 errors, exceeded DFT_PARSE_ERROR_LIMIT)
                                                                                                                                                          5984         if self.check_keyword(keywords::Const) {
                                                                                                                                                          5985             return Err(self.span_fatal(self.span, "extern items cannot be `const`"));
                                                                                                                                                          5986         }

