File_Code/rust/c2c37b401e/mod/mod_after.rs --- 1/2 --- Text (2 errors, exceeded DFT_PARSE_ERROR_LIMIT)
1549                             span: pat.span,                                                                                                             1549                             span: Span { expn_id: self.span.expn_id, ..pat.span },

File_Code/rust/c2c37b401e/mod/mod_after.rs --- 2/2 --- Text (2 errors, exceeded DFT_PARSE_ERROR_LIMIT)
1580         let variant_path = cx.path(variant.span, vec![enum_ident, variant_ident]);                                                                      1580         let sp = Span { expn_id: self.span.expn_id, ..variant.span };
                                                                                                                                                             1581         let variant_path = cx.path(sp, vec![enum_ident, variant_ident]);

