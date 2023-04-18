File_Code/rust/bfbdff0e2d/mod/mod_after.rs --- Rust
913                 struct_span_err!(                                                                                                                        913                 let mut err = struct_span_err!(
914                     self.tcx.sess, span, E0388,                                                                                                          914                     self.tcx.sess, span, E0388,
915                     "{} in a static location", prefix)                                                                                                   915                     "{} in a static location", prefix);
                                                                                                                                                             916                 err.span_label(span, &format!("cannot write data in a static definition"));
                                                                                                                                                             917                 err

