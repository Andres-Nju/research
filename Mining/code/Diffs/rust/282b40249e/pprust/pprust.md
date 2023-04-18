File_Code/rust/282b40249e/pprust/pprust_after.rs --- Text (6 errors, exceeded DFT_PARSE_ERROR_LIMIT)
964         for (i, elt) in elts.iter().enumerate() {                                                                                                        964         let mut i = 0;
...                                                                                                                                                          965         for elt in elts {
965             self.maybe_print_comment(get_span(elt).hi)?;                                                                                                 966             self.maybe_print_comment(get_span(elt).hi)?;
966             op(self, elt)?;                                                                                                                              967             op(self, elt)?;
                                                                                                                                                             968             i += 1;

