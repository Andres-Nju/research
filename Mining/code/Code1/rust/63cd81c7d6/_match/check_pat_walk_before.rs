    pub fn check_pat_walk(
        &self,
        pat: &'gcx hir::Pat,
        mut expected: Ty<'tcx>,
        mut def_bm: ty::BindingMode,
        is_arg: bool)
    {
        let tcx = self.tcx;

        debug!("check_pat_walk(pat={:?},expected={:?},def_bm={:?},is_arg={})",
            pat, expected, def_bm, is_arg);

        let is_non_ref_pat = match pat.node {
            PatKind::Struct(..) |
            PatKind::TupleStruct(..) |
            PatKind::Tuple(..) |
            PatKind::Box(_) |
            PatKind::Range(..) |
            PatKind::Slice(..) => true,
            PatKind::Lit(ref lt) => {
                let ty = self.check_expr(lt);
                match ty.sty {
                    ty::Ref(..) => false,
                    _ => true,
                }
            }
            PatKind::Path(ref qpath) => {
                let (def, _, _) = self.resolve_ty_and_def_ufcs(qpath, pat.id, pat.span);
                match def {
                    Def::Const(..) | Def::AssociatedConst(..) => false,
                    _ => true,
                }
            }
            PatKind::Wild |
            PatKind::Binding(..) |
            PatKind::Ref(..) => false,
        };
        if is_non_ref_pat {
            debug!("pattern is non reference pattern");
            let mut exp_ty = self.resolve_type_vars_with_obligations(&expected);

            // Peel off as many `&` or `&mut` from the discriminant as possible. For example,
            // for `match &&&mut Some(5)` the loop runs three times, aborting when it reaches
            // the `Some(5)` which is not of type Ref.
            //
            // For each ampersand peeled off, update the binding mode and push the original
            // type into the adjustments vector.
            //
            // See the examples in `run-pass/match-defbm*.rs`.
            let mut pat_adjustments = vec![];
            expected = loop {
                debug!("inspecting {:?} with type {:?}", exp_ty, exp_ty.sty);
                match exp_ty.sty {
                    ty::Ref(_, inner_ty, inner_mutability) => {
                        debug!("current discriminant is Ref, inserting implicit deref");
                        // Preserve the reference type. We'll need it later during HAIR lowering.
                        pat_adjustments.push(exp_ty);

                        exp_ty = inner_ty;
                        def_bm = match def_bm {
                            // If default binding mode is by value, make it `ref` or `ref mut`
                            // (depending on whether we observe `&` or `&mut`).
                            ty::BindByValue(_) =>
                                ty::BindByReference(inner_mutability),

                            // Once a `ref`, always a `ref`. This is because a `& &mut` can't mutate
                            // the underlying value.
                            ty::BindByReference(hir::Mutability::MutImmutable) =>
                                ty::BindByReference(hir::Mutability::MutImmutable),

                            // When `ref mut`, stay a `ref mut` (on `&mut`) or downgrade to `ref`
                            // (on `&`).
                            ty::BindByReference(hir::Mutability::MutMutable) =>
                                ty::BindByReference(inner_mutability),
                        };
                    },
                    _ => break exp_ty,
                }
            };
            if pat_adjustments.len() > 0 {
                debug!("default binding mode is now {:?}", def_bm);
                self.inh.tables.borrow_mut()
                    .pat_adjustments_mut()
                    .insert(pat.hir_id, pat_adjustments);
            }
        } else if let PatKind::Ref(..) = pat.node {
            // When you encounter a `&pat` pattern, reset to "by
            // value". This is so that `x` and `y` here are by value,
            // as they appear to be:
            //
            // ```
            // match &(&22, &44) {
            //   (&x, &y) => ...
            // }
            // ```
            //
            // cc #46688
            def_bm = ty::BindByValue(hir::MutImmutable);
        }

        // Lose mutability now that we know binding mode and discriminant type.
        let def_bm = def_bm;
        let expected = expected;

        let ty = match pat.node {
            PatKind::Wild => {
                expected
            }
            PatKind::Lit(ref lt) => {
                // We've already computed the type above (when checking for a non-ref pat), so
                // avoid computing it again.
                let ty = self.node_ty(lt.hir_id);

                // Byte string patterns behave the same way as array patterns
                // They can denote both statically and dynamically sized byte arrays
                let mut pat_ty = ty;
                if let hir::ExprKind::Lit(ref lt) = lt.node {
                    if let ast::LitKind::ByteStr(_) = lt.node {
                        let expected_ty = self.structurally_resolved_type(pat.span, expected);
                        if let ty::Ref(_, r_ty, _) = expected_ty.sty {
                            if let ty::Slice(_) = r_ty.sty {
                                pat_ty = tcx.mk_imm_ref(tcx.types.re_static,
                                                         tcx.mk_slice(tcx.types.u8))
                            }
                        }
                    }
                }

                // somewhat surprising: in this case, the subtyping
                // relation goes the opposite way as the other
                // cases. Actually what we really want is not a subtyping
                // relation at all but rather that there exists a LUB (so
                // that they can be compared). However, in practice,
                // constants are always scalars or strings.  For scalars
                // subtyping is irrelevant, and for strings `ty` is
                // type is `&'static str`, so if we say that
                //
                //     &'static str <: expected
                //
                // that's equivalent to there existing a LUB.
                self.demand_suptype(pat.span, expected, pat_ty);
                pat_ty
            }
            PatKind::Range(ref begin, ref end, _) => {
                let lhs_ty = self.check_expr(begin);
                let rhs_ty = self.check_expr(end);

                // Check that both end-points are of numeric or char type.
                let numeric_or_char = |ty: Ty| ty.is_numeric() || ty.is_char();
                let lhs_compat = numeric_or_char(lhs_ty);
                let rhs_compat = numeric_or_char(rhs_ty);

                if !lhs_compat || !rhs_compat {
                    let span = if !lhs_compat && !rhs_compat {
                        pat.span
                    } else if !lhs_compat {
                        begin.span
                    } else {
                        end.span
                    };

                    let mut err = struct_span_err!(
                        tcx.sess,
                        span,
                        E0029,
                        "only char and numeric types are allowed in range patterns"
                    );
                    err.span_label(span, "ranges require char or numeric types");
                    err.note(&format!("start type: {}", self.ty_to_string(lhs_ty)));
                    err.note(&format!("end type: {}", self.ty_to_string(rhs_ty)));
                    if tcx.sess.teach(&err.get_code().unwrap()) {
                        err.note(
                            "In a match expression, only numbers and characters can be matched \
                             against a range. This is because the compiler checks that the range \
                             is non-empty at compile-time, and is unable to evaluate arbitrary \
                             comparison functions. If you want to capture values of an orderable \
                             type between two end-points, you can use a guard."
                         );
                    }
                    err.emit();
                    return;
                }

                // Now that we know the types can be unified we find the unified type and use
                // it to type the entire expression.
                let common_type = self.resolve_type_vars_if_possible(&lhs_ty);

                // subtyping doesn't matter here, as the value is some kind of scalar
                self.demand_eqtype(pat.span, expected, lhs_ty);
                self.demand_eqtype(pat.span, expected, rhs_ty);
                common_type
            }
            PatKind::Binding(ba, var_id, _, ref sub) => {
                let bm = if ba == hir::BindingAnnotation::Unannotated {
                    def_bm
                } else {
                    ty::BindingMode::convert(ba)
                };
                self.inh
                    .tables
                    .borrow_mut()
                    .pat_binding_modes_mut()
                    .insert(pat.hir_id, bm);
                debug!("check_pat_walk: pat.hir_id={:?} bm={:?}", pat.hir_id, bm);
                let typ = self.local_ty(pat.span, pat.id);
                match bm {
                    ty::BindByReference(mutbl) => {
                        // if the binding is like
                        //    ref x | ref const x | ref mut x
                        // then `x` is assigned a value of type `&M T` where M is the mutability
                        // and T is the expected type.
                        let region_var = self.next_region_var(infer::PatternRegion(pat.span));
                        let mt = ty::TypeAndMut { ty: expected, mutbl: mutbl };
                        let region_ty = tcx.mk_ref(region_var, mt);

                        // `x` is assigned a value of type `&M T`, hence `&M T <: typeof(x)` is
                        // required. However, we use equality, which is stronger. See (*) for
                        // an explanation.
                        self.demand_eqtype(pat.span, region_ty, typ);
                    }
                    // otherwise the type of x is the expected type T
                    ty::BindByValue(_) => {
                        // As above, `T <: typeof(x)` is required but we
                        // use equality, see (*) below.
                        self.demand_eqtype(pat.span, expected, typ);
                    }
                }

                // if there are multiple arms, make sure they all agree on
                // what the type of the binding `x` ought to be
                if var_id != pat.id {
                    let vt = self.local_ty(pat.span, var_id);
                    self.demand_eqtype(pat.span, vt, typ);
                }

                if let Some(ref p) = *sub {
                    self.check_pat_walk(&p, expected, def_bm, true);
                }

                typ
            }
            PatKind::TupleStruct(ref qpath, ref subpats, ddpos) => {
                self.check_pat_tuple_struct(pat, qpath, &subpats, ddpos, expected, def_bm)
            }
            PatKind::Path(ref qpath) => {
                self.check_pat_path(pat, qpath, expected)
            }
            PatKind::Struct(ref qpath, ref fields, etc) => {
                self.check_pat_struct(pat, qpath, fields, etc, expected, def_bm)
            }
            PatKind::Tuple(ref elements, ddpos) => {
                let mut expected_len = elements.len();
                if ddpos.is_some() {
                    // Require known type only when `..` is present
                    if let ty::Tuple(ref tys) =
                            self.structurally_resolved_type(pat.span, expected).sty {
                        expected_len = tys.len();
                    }
                }
                let max_len = cmp::max(expected_len, elements.len());

                let element_tys_iter = (0..max_len).map(|_| self.next_ty_var(
                    // FIXME: MiscVariable for now, obtaining the span and name information
                    //       from all tuple elements isn't trivial.
                    TypeVariableOrigin::TypeInference(pat.span)));
                let element_tys = tcx.mk_type_list(element_tys_iter);
                let pat_ty = tcx.mk_ty(ty::Tuple(element_tys));
                if let Some(mut err) = self.demand_eqtype_diag(pat.span, expected, pat_ty) {
                    err.emit();
                    let element_tys_iter = (0..max_len).map(|_| tcx.types.err);
                    for (_, elem) in elements.iter().enumerate_and_adjust(max_len, ddpos) {
                        self.check_pat_walk(elem, &tcx.types.err, def_bm, true);
                    }
                    tcx.mk_tup(element_tys_iter)
                } else {
                    for (i, elem) in elements.iter().enumerate_and_adjust(max_len, ddpos) {
                        self.check_pat_walk(elem, &element_tys[i], def_bm, true);
                    }
                    pat_ty
                }
            }
            PatKind::Box(ref inner) => {
                let inner_ty = self.next_ty_var(TypeVariableOrigin::TypeInference(inner.span));
                let uniq_ty = tcx.mk_box(inner_ty);

                if self.check_dereferencable(pat.span, expected, &inner) {
                    // Here, `demand::subtype` is good enough, but I don't
                    // think any errors can be introduced by using
                    // `demand::eqtype`.
                    self.demand_eqtype(pat.span, expected, uniq_ty);
                    self.check_pat_walk(&inner, inner_ty, def_bm, true);
                    uniq_ty
                } else {
                    self.check_pat_walk(&inner, tcx.types.err, def_bm, true);
                    tcx.types.err
                }
            }
            PatKind::Ref(ref inner, mutbl) => {
                let expected = self.shallow_resolve(expected);
                if self.check_dereferencable(pat.span, expected, &inner) {
                    // `demand::subtype` would be good enough, but using
                    // `eqtype` turns out to be equally general. See (*)
                    // below for details.

                    // Take region, inner-type from expected type if we
                    // can, to avoid creating needless variables.  This
                    // also helps with the bad interactions of the given
                    // hack detailed in (*) below.
                    debug!("check_pat_walk: expected={:?}", expected);
                    let (rptr_ty, inner_ty) = match expected.sty {
                        ty::Ref(_, r_ty, r_mutbl) if r_mutbl == mutbl => {
                            (expected, r_ty)
                        }
                        _ => {
                            let inner_ty = self.next_ty_var(
                                TypeVariableOrigin::TypeInference(inner.span));
                            let mt = ty::TypeAndMut { ty: inner_ty, mutbl: mutbl };
                            let region = self.next_region_var(infer::PatternRegion(pat.span));
                            let rptr_ty = tcx.mk_ref(region, mt);
                            debug!("check_pat_walk: demanding {:?} = {:?}", expected, rptr_ty);
                            let err = self.demand_eqtype_diag(pat.span, expected, rptr_ty);

                            // Look for a case like `fn foo(&foo: u32)` and suggest
                            // `fn foo(foo: &u32)`
                            if let Some(mut err) = err {
                                if is_arg {
                                    if let PatKind::Binding(..) = inner.node {
                                        if let Ok(snippet) = tcx.sess.source_map()
                                                                     .span_to_snippet(pat.span)
                                        {
                                            err.help(&format!("did you mean `{}: &{}`?",
                                                              &snippet[1..],
                                                              expected));
                                        }
                                    }
                                }
                                err.emit();
                            }
                            (rptr_ty, inner_ty)
                        }
                    };

                    self.check_pat_walk(&inner, inner_ty, def_bm, true);
                    rptr_ty
                } else {
                    self.check_pat_walk(&inner, tcx.types.err, def_bm, true);
                    tcx.types.err
                }
            }
            PatKind::Slice(ref before, ref slice, ref after) => {
                let expected_ty = self.structurally_resolved_type(pat.span, expected);
                let (inner_ty, slice_ty) = match expected_ty.sty {
                    ty::Array(inner_ty, size) => {
                        let size = size.unwrap_usize(tcx);
                        let min_len = before.len() as u64 + after.len() as u64;
                        if slice.is_none() {
                            if min_len != size {
                                struct_span_err!(
                                    tcx.sess, pat.span, E0527,
                                    "pattern requires {} elements but array has {}",
                                    min_len, size)
                                    .span_label(pat.span, format!("expected {} elements",size))
                                    .emit();
                            }
                            (inner_ty, tcx.types.err)
                        } else if let Some(rest) = size.checked_sub(min_len) {
                            (inner_ty, tcx.mk_array(inner_ty, rest))
                        } else {
                            struct_span_err!(tcx.sess, pat.span, E0528,
                                    "pattern requires at least {} elements but array has {}",
                                    min_len, size)
                                .span_label(pat.span,
                                    format!("pattern cannot match array of {} elements", size))
                                .emit();
                            (inner_ty, tcx.types.err)
                        }
                    }
                    ty::Slice(inner_ty) => (inner_ty, expected_ty),
                    _ => {
                        if !expected_ty.references_error() {
                            let mut err = struct_span_err!(
                                tcx.sess, pat.span, E0529,
                                "expected an array or slice, found `{}`",
                                expected_ty);
                            if let ty::Ref(_, ty, _) = expected_ty.sty {
                                match ty.sty {
                                    ty::Array(..) | ty::Slice(..) => {
                                        err.help("the semantics of slice patterns changed \
                                                  recently; see issue #23121");
                                    }
                                    _ => {}
                                }
                            }

                            err.span_label( pat.span,
                                format!("pattern cannot match with input type `{}`", expected_ty)
                            ).emit();
                        }
                        (tcx.types.err, tcx.types.err)
                    }
                };

                for elt in before {
                    self.check_pat_walk(&elt, inner_ty, def_bm, true);
                }
                if let Some(ref slice) = *slice {
                    self.check_pat_walk(&slice, slice_ty, def_bm, true);
                }
                for elt in after {
                    self.check_pat_walk(&elt, inner_ty, def_bm, true);
                }
                expected_ty
            }
        };

        self.write_ty(pat.hir_id, ty);

        // (*) In most of the cases above (literals and constants being
        // the exception), we relate types using strict equality, even
        // though subtyping would be sufficient. There are a few reasons
        // for this, some of which are fairly subtle and which cost me
        // (nmatsakis) an hour or two debugging to remember, so I thought
        // I'd write them down this time.
        //
        // 1. There is no loss of expressiveness here, though it does
        // cause some inconvenience. What we are saying is that the type
        // of `x` becomes *exactly* what is expected. This can cause unnecessary
        // errors in some cases, such as this one:
        //
        // ```
        // fn foo<'x>(x: &'x int) {
        //    let a = 1;
        //    let mut z = x;
        //    z = &a;
        // }
        // ```
        //
        // The reason we might get an error is that `z` might be
        // assigned a type like `&'x int`, and then we would have
        // a problem when we try to assign `&a` to `z`, because
        // the lifetime of `&a` (i.e., the enclosing block) is
        // shorter than `'x`.
        //
        // HOWEVER, this code works fine. The reason is that the
        // expected type here is whatever type the user wrote, not
        // the initializer's type. In this case the user wrote
        // nothing, so we are going to create a type variable `Z`.
        // Then we will assign the type of the initializer (`&'x
        // int`) as a subtype of `Z`: `&'x int <: Z`. And hence we
        // will instantiate `Z` as a type `&'0 int` where `'0` is
        // a fresh region variable, with the constraint that `'x :
        // '0`.  So basically we're all set.
        //
        // Note that there are two tests to check that this remains true
        // (`regions-reassign-{match,let}-bound-pointer.rs`).
        //
        // 2. Things go horribly wrong if we use subtype. The reason for
        // THIS is a fairly subtle case involving bound regions. See the
        // `givens` field in `region_constraints`, as well as the test
        // `regions-relate-bound-regions-on-closures-to-inference-variables.rs`,
        // for details. Short version is that we must sometimes detect
        // relationships between specific region variables and regions
        // bound in a closure signature, and that detection gets thrown
        // off when we substitute fresh region variables here to enable
        // subtyping.
    }
