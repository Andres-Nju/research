fn equate_intrinsic_type<'a, 'tcx>(ccx: &CrateCtxt<'a, 'tcx>,
                                   it: &hir::ForeignItem,
                                   n_tps: usize,
                                   abi: Abi,
                                   inputs: Vec<ty::Ty<'tcx>>,
                                   output: ty::FnOutput<'tcx>) {
    let tcx = ccx.tcx;
    let def_id = tcx.map.local_def_id(it.id);
    let i_ty = tcx.lookup_item_type(def_id);

    let mut substs = Substs::empty();
    substs.types = i_ty.generics.types.map(|def| tcx.mk_param_from_def(def));

    let fty = tcx.mk_fn_def(def_id, tcx.mk_substs(substs),
                            tcx.mk_bare_fn(ty::BareFnTy {
        unsafety: hir::Unsafety::Unsafe,
        abi: abi,
        sig: ty::Binder(FnSig {
            inputs: inputs,
            output: output,
            variadic: false,
        }),
    }));
    let i_n_tps = i_ty.generics.types.len(subst::FnSpace);
    if i_n_tps != n_tps {
        struct_span_err!(tcx.sess, it.span, E0094,
            "intrinsic has wrong number of type \
             parameters: found {}, expected {}",
             i_n_tps, n_tps)
             .span_label(it.span, &format!("expected {} type parameter", n_tps))
             .emit();
    } else {
        require_same_types(ccx,
                           TypeOrigin::IntrinsicType(it.span),
                           i_ty.ty,
                           fty);
    }
}
