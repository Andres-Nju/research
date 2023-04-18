File_Code/rust/b564c6a5e4/callee/callee_after.rs --- Rust
31         span_err!(ccx.tcx.sess, span, E0040, "explicit use of destructor method");                                                                        31         struct_span_err!(ccx.tcx.sess, span, E0040, "explicit use of destructor method")
                                                                                                                                                             32             .span_label(span, &format!("call to destructor method"))
                                                                                                                                                             33             .emit();

