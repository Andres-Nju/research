pub fn check_legal_trait_for_method_call(ccx: &CrateCtxt, span: Span, trait_id: DefId) {
    if ccx.tcx.lang_items.drop_trait() == Some(trait_id) {
        struct_span_err!(ccx.tcx.sess, span, E0040, "explicit use of destructor method")
            .span_label(span, &format!("call to destructor method"))
            .emit();
    }
}
