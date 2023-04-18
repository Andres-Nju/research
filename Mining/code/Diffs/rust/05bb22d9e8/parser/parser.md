File_Code/rust/05bb22d9e8/parser/parser_after.rs --- Text (6 errors, exceeded DFT_PARSE_ERROR_LIMIT)
                                                                                                                                                           775                 _ if self.prev_span == syntax_pos::DUMMY_SP => {
                                                                                                                                                           776                     // Account for macro context where the previous span might not be
                                                                                                                                                           777                     // available to avoid incorrect output (#54841).
                                                                                                                                                           778                     err.span_label(self.span, "unexpected token");
                                                                                                                                                           779                 }

