File_Code/rust/c89e27824f/astconv_util/astconv_util_after.rs --- Rust
27                 span_err!(self.sess, typ.span, E0109,                                                                                                     27                 struct_span_err!(self.sess, typ.span, E0109,
28                           "type parameters are not allowed on this type");                                                                                28                                  "type parameters are not allowed on this type")
                                                                                                                                                             29                     .span_label(typ.span, &format!("type parameter not allowed"))
                                                                                                                                                             30                     .emit();

