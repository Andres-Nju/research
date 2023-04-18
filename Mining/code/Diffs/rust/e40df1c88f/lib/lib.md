File_Code/rust/e40df1c88f/lib/lib_after.rs --- Rust
  .                                                                                                                                                          264                             let sp = if let Some(sp) = ps.span() { sp } else { start_span };
264                             struct_span_err!(tcx.sess, start_span, E0132,                                                                                265                             struct_span_err!(tcx.sess, sp, E0132,
265                                 "start function is not allowed to have type parameters")                                                                 266                                 "start function is not allowed to have type parameters")
266                                 .span_label(ps.span().unwrap(),                                                                                          267                                 .span_label(sp,

