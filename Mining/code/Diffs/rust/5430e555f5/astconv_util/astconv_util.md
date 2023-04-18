File_Code/rust/5430e555f5/astconv_util/astconv_util_after.rs --- Rust
32                 span_err!(self.sess, lifetime.span, E0110,                                                                                                32                 struct_span_err!(self.sess, lifetime.span, E0110,
33                           "lifetime parameters are not allowed on this type");                                                                            33                                  "lifetime parameters are not allowed on this type")
                                                                                                                                                             34                     .span_label(lifetime.span,
                                                                                                                                                             35                                 &format!("lifetime parameter not allowed on this type"))
                                                                                                                                                             36                     .emit();

