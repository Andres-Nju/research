File_Code/rust/fbb099edca/borrow_check/borrow_check_after.rs --- Rust
53     if !tcx.has_attr(def_id, "rustc_mir_borrowck") || !tcx.sess.opts.debugging_opts.borrowck_mir {                                                        53     if !tcx.has_attr(def_id, "rustc_mir_borrowck") && !tcx.sess.opts.debugging_opts.borrowck_mir {

