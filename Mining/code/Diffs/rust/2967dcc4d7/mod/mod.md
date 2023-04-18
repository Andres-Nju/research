File_Code/rust/2967dcc4d7/mod/mod_after.rs --- Rust
351                     span_err!(tcx.sess, span, E0184,                                                                                                     351                     struct_span_err!(tcx.sess, span, E0184,
352                               "the trait `Copy` may not be implemented for this type; \                                                                  352                               "the trait `Copy` may not be implemented for this type; \
353                                the type has a destructor");                                                                                              353                                the type has a destructor")
                                                                                                                                                             354                         .span_label(span, &format!("Copy not allowed on types with destructors"))
                                                                                                                                                             355                         .emit();

