File_Code/rust/cd56d47da3/check_attr/check_attr_after.rs --- Rust
45             span_err!(self.sess, attr.span, E0518, "attribute should be applied to function");                                                            45             struct_span_err!(self.sess, attr.span, E0518, "attribute should be applied to function")
                                                                                                                                                             46                 .span_label(attr.span, &format!("requires a function"))
                                                                                                                                                             47                 .emit();

