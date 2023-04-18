fn write_substs_to_tcx<'a, 'tcx>(ccx: &CrateCtxt<'a, 'tcx>,
                                 node_id: ast::NodeId,
                                 item_substs: ty::ItemSubsts<'tcx>) {
    if !item_substs.is_noop() {
        debug!("write_substs_to_tcx({}, {:?})",
               node_id,
               item_substs);

        assert!(!item_substs.substs.types.needs_infer());

        ccx.tcx.tables.borrow_mut().item_substs.insert(node_id, item_substs);
    }
}

fn require_c_abi_if_variadic(tcx: TyCtxt,
                             decl: &hir::FnDecl,
                             abi: Abi,
                             span: Span) {
    if decl.variadic && abi != Abi::C {
        span_err!(tcx.sess, span, E0045,
                  "variadic function must have C calling convention");
    }
}

fn require_same_types<'a, 'tcx>(ccx: &CrateCtxt<'a, 'tcx>,
                                origin: TypeOrigin,
                                t1: Ty<'tcx>,
                                t2: Ty<'tcx>)
                                -> bool {
    ccx.tcx.infer_ctxt(None, None, ProjectionMode::AnyFinal).enter(|infcx| {
        if let Err(err) = infcx.eq_types(false, origin.clone(), t1, t2) {
            infcx.report_mismatched_types(origin, t1, t2, err);
            false
        } else {
            true
        }
    })
}

fn check_main_fn_ty(ccx: &CrateCtxt,
                    main_id: ast::NodeId,
                    main_span: Span) {
    let tcx = ccx.tcx;
    let main_t = tcx.node_id_to_type(main_id);
    match main_t.sty {
        ty::TyFnDef(..) => {
            match tcx.map.find(main_id) {
                Some(hir_map::NodeItem(it)) => {
                    match it.node {
                        hir::ItemFn(_, _, _, _, ref ps, _)
                        if ps.is_parameterized() => {
                            span_err!(ccx.tcx.sess, main_span, E0131,
                                      "main function is not allowed to have type parameters");
                            return;
                        }
                        _ => ()
                    }
                }
                _ => ()
            }
            let main_def_id = tcx.map.local_def_id(main_id);
            let substs = tcx.mk_substs(Substs::empty());
            let se_ty = tcx.mk_fn_def(main_def_id, substs,
                                      tcx.mk_bare_fn(ty::BareFnTy {
                unsafety: hir::Unsafety::Normal,
                abi: Abi::Rust,
                sig: ty::Binder(ty::FnSig {
                    inputs: Vec::new(),
                    output: ty::FnConverging(tcx.mk_nil()),
                    variadic: false
                })
            }));

            require_same_types(
                ccx,
                TypeOrigin::MainFunctionType(main_span),
                main_t,
                se_ty);
        }
        _ => {
            span_bug!(main_span,
                      "main has a non-function type: found `{}`",
                      main_t);
        }
    }
}

fn check_start_fn_ty(ccx: &CrateCtxt,
                     start_id: ast::NodeId,
                     start_span: Span) {
    let tcx = ccx.tcx;
    let start_t = tcx.node_id_to_type(start_id);
    match start_t.sty {
        ty::TyFnDef(..) => {
            match tcx.map.find(start_id) {
                Some(hir_map::NodeItem(it)) => {
                    match it.node {
                        hir::ItemFn(_,_,_,_,ref ps,_)
                        if ps.is_parameterized() => {
                            let sp = if let Some(sp) = ps.span() { sp } else { start_span };
                            struct_span_err!(tcx.sess, sp, E0132,
                                "start function is not allowed to have type parameters")
                                .span_label(sp,
                                            &format!("start function cannot have type parameters"))
                                .emit();
                            return;
                        }
                        _ => ()
                    }
                }
                _ => ()
            }

            let start_def_id = ccx.tcx.map.local_def_id(start_id);
            let substs = tcx.mk_substs(Substs::empty());
            let se_ty = tcx.mk_fn_def(start_def_id, substs,
                                      tcx.mk_bare_fn(ty::BareFnTy {
                unsafety: hir::Unsafety::Normal,
                abi: Abi::Rust,
                sig: ty::Binder(ty::FnSig {
                    inputs: vec!(
                        tcx.types.isize,
                        tcx.mk_imm_ptr(tcx.mk_imm_ptr(tcx.types.u8))
                    ),
                    output: ty::FnConverging(tcx.types.isize),
                    variadic: false,
                }),
            }));

            require_same_types(
                ccx,
                TypeOrigin::StartFunctionType(start_span),
                start_t,
                se_ty);
        }
        _ => {
            span_bug!(start_span,
                      "start has a non-function type: found `{}`",
                      start_t);
        }
    }
}

fn check_for_entry_fn(ccx: &CrateCtxt) {
    let tcx = ccx.tcx;
    let _task = tcx.dep_graph.in_task(DepNode::CheckEntryFn);
    if let Some((id, sp)) = *tcx.sess.entry_fn.borrow() {
        match tcx.sess.entry_type.get() {
            Some(config::EntryMain) => check_main_fn_ty(ccx, id, sp),
            Some(config::EntryStart) => check_start_fn_ty(ccx, id, sp),
            Some(config::EntryNone) => {}
            None => bug!("entry function without a type")
        }
    }
}

pub fn check_crate<'a, 'tcx>(tcx: TyCtxt<'a, 'tcx, 'tcx>,
                             trait_map: hir::TraitMap)
                             -> CompileResult {
    let time_passes = tcx.sess.time_passes();
    let ccx = CrateCtxt {
        ast_ty_to_ty_cache: RefCell::new(NodeMap()),
        trait_map: trait_map,
        all_traits: RefCell::new(None),
        stack: RefCell::new(Vec::new()),
        tcx: tcx
    };

    // this ensures that later parts of type checking can assume that items
    // have valid types and not error
    tcx.sess.track_errors(|| {
        time(time_passes, "type collecting", ||
             collect::collect_item_types(&ccx));

    })?;

    time(time_passes, "variance inference", ||
         variance::infer_variance(tcx));

    tcx.sess.track_errors(|| {
      time(time_passes, "coherence checking", ||
          coherence::check_coherence(&ccx));
    })?;

    time(time_passes, "wf checking", || check::check_wf_new(&ccx))?;

    time(time_passes, "item-types checking", || check::check_item_types(&ccx))?;

    time(time_passes, "item-bodies checking", || check::check_item_bodies(&ccx))?;

    time(time_passes, "drop-impl checking", || check::check_drop_impls(&ccx))?;

    check_unused::check_crate(tcx);
    check_for_entry_fn(&ccx);

    let err_count = tcx.sess.err_count();
    if err_count == 0 {
        Ok(())
    } else {
        Err(err_count)
    }
