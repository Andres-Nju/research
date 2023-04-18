File_Code/rust/046482e95e/error_reporting/error_reporting_after.rs --- Rust
849                     None => return,                                                                                                                      849                     None => {
                                                                                                                                                             850                         self.tcx.sess.delay_span_bug(span,
                                                                                                                                                             851                             &format!("constant in type had an ignored error: {:?}", err));
                                                                                                                                                             852                         return;
                                                                                                                                                             853                     }

