File_Code/rust/6d998d664b/wfcheck/wfcheck_after.rs --- Rust
656     struct_span_err!(ccx.tcx.sess, span, E0392,                                                                                                          656     let mut err = struct_span_err!(ccx.tcx.sess, span, E0392,
657                      "parameter `{}` is never used", param_name)                                                                                         657                   "parameter `{}` is never used", param_name);
                                                                                                                                                             658     err.span_label(span, &format!("unused type parameter"));
                                                                                                                                                             659     err

