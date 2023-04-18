File_Code/rust/c44ffafab9/impl_wf_check/impl_wf_check_after.rs --- Rust
105         tcx.sess.delay_span_bug(tcx.def_span(impl_def_id), &format(                                                                                      105         tcx.sess.delay_span_bug(
...                                                                                                                                                          106             tcx.def_span(impl_def_id),
106             "potentially unconstrained type parameters weren't evaluated on `{:?}`",                                                                     107             "potentially unconstrained type parameters weren't evaluated",
107             impl_self_ty,                                                                                                                                ... 
108         ));                                                                                                                                              108         );

