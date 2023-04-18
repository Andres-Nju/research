File_Code/rust/85388f0958/intrinsic/intrinsic_after.rs --- Rust
54         span_err!(tcx.sess, it.span, E0094,                                                                                                               54         struct_span_err!(tcx.sess, it.span, E0094,
55             "intrinsic has wrong number of type \                                                                                                         55             "intrinsic has wrong number of type \
56              parameters: found {}, expected {}",                                                                                                          56              parameters: found {}, expected {}",
57              i_n_tps, n_tps);                                                                                                                             57              i_n_tps, n_tps)
                                                                                                                                                             58              .span_label(it.span, &format!("expected {} type parameter", n_tps))
                                                                                                                                                             59              .emit();

