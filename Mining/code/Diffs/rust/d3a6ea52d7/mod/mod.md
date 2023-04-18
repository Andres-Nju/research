File_Code/rust/d3a6ea52d7/mod/mod_after.rs --- Text (12 errors, exceeded DFT_PARSE_ERROR_LIMIT)
1231                 span_err!(tcx.sess, sp, E0076, "SIMD vector should be homogeneous");                                                                    1231                 struct_span_err!(tcx.sess, sp, E0076, "SIMD vector should be homogeneous")
                                                                                                                                                             1232                                 .span_label(sp, &format!("SIMD elements must have the same type"))
                                                                                                                                                             1233                                 .emit();

