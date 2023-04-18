File_Code/rust/ee9a4720ee/encoder/encoder_after.rs --- Rust
933                 let needs_inline = types > 0 || tcx.trans_fn_attrs(def_id).requests_inline() &&                                                          933                 let needs_inline = (types > 0 || tcx.trans_fn_attrs(def_id).requests_inline()) &&

