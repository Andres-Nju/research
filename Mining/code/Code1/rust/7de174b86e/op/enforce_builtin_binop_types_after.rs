    fn enforce_builtin_binop_types(
        &self,
        lhs_expr: &'tcx hir::Expr<'tcx>,
        lhs_ty: Ty<'tcx>,
        rhs_expr: &'tcx hir::Expr<'tcx>,
        rhs_ty: Ty<'tcx>,
        op: hir::BinOp,
    ) -> Ty<'tcx> {
        debug_assert!(is_builtin_binop(lhs_ty, rhs_ty, op));

        let tcx = self.tcx;
        match BinOpCategory::from(op) {
            BinOpCategory::Shortcircuit => {
                self.demand_suptype(lhs_expr.span, tcx.mk_bool(), lhs_ty);
                self.demand_suptype(rhs_expr.span, tcx.mk_bool(), rhs_ty);
                tcx.mk_bool()
            }

            BinOpCategory::Shift => {
                // result type is same as LHS always
                lhs_ty
            }

            BinOpCategory::Math | BinOpCategory::Bitwise => {
                // both LHS and RHS and result will have the same type
                self.demand_suptype(rhs_expr.span, lhs_ty, rhs_ty);
                lhs_ty
            }

            BinOpCategory::Comparison => {
                // both LHS and RHS and result will have the same type
                self.demand_suptype(rhs_expr.span, lhs_ty, rhs_ty);
                tcx.mk_bool()
            }
        }
    }

    fn check_overloaded_binop(
        &self,
        expr: &'tcx hir::Expr<'tcx>,
        lhs_expr: &'tcx hir::Expr<'tcx>,
        rhs_expr: &'tcx hir::Expr<'tcx>,
        op: hir::BinOp,
        is_assign: IsAssign,
    ) -> (Ty<'tcx>, Ty<'tcx>, Ty<'tcx>) {
        debug!(
            "check_overloaded_binop(expr.hir_id={}, op={:?}, is_assign={:?})",
            expr.hir_id, op, is_assign
        );

        let lhs_ty = match is_assign {
            IsAssign::No => {
                // Find a suitable supertype of the LHS expression's type, by coercing to
                // a type variable, to pass as the `Self` to the trait, avoiding invariant
                // trait matching creating lifetime constraints that are too strict.
                // e.g., adding `&'a T` and `&'b T`, given `&'x T: Add<&'x T>`, will result
                // in `&'a T <: &'x T` and `&'b T <: &'x T`, instead of `'a = 'b = 'x`.
                let lhs_ty = self.check_expr_with_needs(lhs_expr, Needs::None);
                let fresh_var = self.next_ty_var(TypeVariableOrigin {
                    kind: TypeVariableOriginKind::MiscVariable,
                    span: lhs_expr.span,
                });
                self.demand_coerce(lhs_expr, lhs_ty, fresh_var, AllowTwoPhase::No)
            }
            IsAssign::Yes => {
                // rust-lang/rust#52126: We have to use strict
                // equivalence on the LHS of an assign-op like `+=`;
                // overwritten or mutably-borrowed places cannot be
                // coerced to a supertype.
                self.check_expr_with_needs(lhs_expr, Needs::MutPlace)
            }
        };
        let lhs_ty = self.resolve_vars_with_obligations(lhs_ty);

        // N.B., as we have not yet type-checked the RHS, we don't have the
        // type at hand. Make a variable to represent it. The whole reason
        // for this indirection is so that, below, we can check the expr
        // using this variable as the expected type, which sometimes lets
        // us do better coercions than we would be able to do otherwise,
        // particularly for things like `String + &String`.
        let rhs_ty_var = self.next_ty_var(TypeVariableOrigin {
            kind: TypeVariableOriginKind::MiscVariable,
            span: rhs_expr.span,
        });

        let result = self.lookup_op_method(lhs_ty, &[rhs_ty_var], Op::Binary(op, is_assign));

        // see `NB` above
        let rhs_ty = self.check_expr_coercable_to_type(rhs_expr, rhs_ty_var);
        let rhs_ty = self.resolve_vars_with_obligations(rhs_ty);

        let return_ty = match result {
            Ok(method) => {
                let by_ref_binop = !op.node.is_by_value();
                if is_assign == IsAssign::Yes || by_ref_binop {
                    if let ty::Ref(region, _, mutbl) = method.sig.inputs()[0].kind {
                        let mutbl = match mutbl {
                            hir::Mutability::Not => AutoBorrowMutability::Not,
                            hir::Mutability::Mut => AutoBorrowMutability::Mut {
                                // Allow two-phase borrows for binops in initial deployment
                                // since they desugar to methods
                                allow_two_phase_borrow: AllowTwoPhase::Yes,
                            },
                        };
                        let autoref = Adjustment {
                            kind: Adjust::Borrow(AutoBorrow::Ref(region, mutbl)),
                            target: method.sig.inputs()[0],
                        };
                        self.apply_adjustments(lhs_expr, vec![autoref]);
                    }
                }
                if by_ref_binop {
                    if let ty::Ref(region, _, mutbl) = method.sig.inputs()[1].kind {
                        let mutbl = match mutbl {
                            hir::Mutability::Not => AutoBorrowMutability::Not,
                            hir::Mutability::Mut => AutoBorrowMutability::Mut {
                                // Allow two-phase borrows for binops in initial deployment
                                // since they desugar to methods
                                allow_two_phase_borrow: AllowTwoPhase::Yes,
                            },
                        };
                        let autoref = Adjustment {
                            kind: Adjust::Borrow(AutoBorrow::Ref(region, mutbl)),
                            target: method.sig.inputs()[1],
                        };
                        // HACK(eddyb) Bypass checks due to reborrows being in
                        // some cases applied on the RHS, on top of which we need
                        // to autoref, which is not allowed by apply_adjustments.
                        // self.apply_adjustments(rhs_expr, vec![autoref]);
                        self.tables
                            .borrow_mut()
                            .adjustments_mut()
                            .entry(rhs_expr.hir_id)
                            .or_default()
                            .push(autoref);
                    }
                }
                self.write_method_call(expr.hir_id, method);

                method.sig.output()
            }
            Err(()) => {
                // error types are considered "builtin"
                if !lhs_ty.references_error() {
                    let source_map = self.tcx.sess.source_map();
                    match is_assign {
                        IsAssign::Yes => {
                            let mut err = struct_span_err!(
                                self.tcx.sess,
                                expr.span,
                                E0368,
                                "binary assignment operation `{}=` cannot be applied to type `{}`",
                                op.node.as_str(),
                                lhs_ty,
                            );
                            err.span_label(
                                lhs_expr.span,
                                format!("cannot use `{}=` on type `{}`", op.node.as_str(), lhs_ty),
                            );
                            let mut suggested_deref = false;
                            if let Ref(_, rty, _) = lhs_ty.kind {
                                if {
                                    self.infcx.type_is_copy_modulo_regions(
                                        self.param_env,
                                        rty,
                                        lhs_expr.span,
                                    ) && self
                                        .lookup_op_method(rty, &[rhs_ty], Op::Binary(op, is_assign))
                                        .is_ok()
                                } {
                                    if let Ok(lstring) = source_map.span_to_snippet(lhs_expr.span) {
                                        let msg = &format!(
                                            "`{}=` can be used on '{}', you can dereference `{}`",
                                            op.node.as_str(),
                                            rty.peel_refs(),
                                            lstring,
                                        );
                                        err.span_suggestion(
                                            lhs_expr.span,
                                            msg,
                                            format!("*{}", lstring),
                                            errors::Applicability::MachineApplicable,
                                        );
                                        suggested_deref = true;
                                    }
                                }
                            }
                            let missing_trait = match op.node {
                                hir::BinOpKind::Add => Some("std::ops::AddAssign"),
                                hir::BinOpKind::Sub => Some("std::ops::SubAssign"),
                                hir::BinOpKind::Mul => Some("std::ops::MulAssign"),
                                hir::BinOpKind::Div => Some("std::ops::DivAssign"),
                                hir::BinOpKind::Rem => Some("std::ops::RemAssign"),
                                hir::BinOpKind::BitAnd => Some("std::ops::BitAndAssign"),
                                hir::BinOpKind::BitXor => Some("std::ops::BitXorAssign"),
                                hir::BinOpKind::BitOr => Some("std::ops::BitOrAssign"),
                                hir::BinOpKind::Shl => Some("std::ops::ShlAssign"),
                                hir::BinOpKind::Shr => Some("std::ops::ShrAssign"),
                                _ => None,
                            };
                            if let Some(missing_trait) = missing_trait {
                                if op.node == hir::BinOpKind::Add
                                    && self.check_str_addition(
                                        lhs_expr, rhs_expr, lhs_ty, rhs_ty, &mut err, true, op,
                                    )
                                {
                                    // This has nothing here because it means we did string
                                    // concatenation (e.g., "Hello " += "World!"). This means
                                    // we don't want the note in the else clause to be emitted
                                } else if let ty::Param(_) = lhs_ty.kind {
                                    // FIXME: point to span of param
                                    err.note(&format!(
                                        "`{}` might need a bound for `{}`",
                                        lhs_ty, missing_trait
                                    ));
                                } else if !suggested_deref {
                                    err.note(&format!(
                                        "an implementation of `{}` might \
                                         be missing for `{}`",
                                        missing_trait, lhs_ty
                                    ));
                                }
                            }
                            err.emit();
                        }
                        IsAssign::No => {
                            let (message, missing_trait) = match op.node {
                                hir::BinOpKind::Add => (
                                    format!("cannot add `{}` to `{}`", rhs_ty, lhs_ty),
                                    Some("std::ops::Add"),
                                ),
                                hir::BinOpKind::Sub => (
                                    format!("cannot subtract `{}` from `{}`", rhs_ty, lhs_ty),
                                    Some("std::ops::Sub"),
                                ),
                                hir::BinOpKind::Mul => (
                                    format!("cannot multiply `{}` to `{}`", rhs_ty, lhs_ty),
                                    Some("std::ops::Mul"),
                                ),
                                hir::BinOpKind::Div => (
                                    format!("cannot divide `{}` by `{}`", lhs_ty, rhs_ty),
                                    Some("std::ops::Div"),
                                ),
                                hir::BinOpKind::Rem => (
                                    format!("cannot mod `{}` by `{}`", lhs_ty, rhs_ty),
                                    Some("std::ops::Rem"),
                                ),
                                hir::BinOpKind::BitAnd => (
                                    format!("no implementation for `{} & {}`", lhs_ty, rhs_ty),
                                    Some("std::ops::BitAnd"),
                                ),
                                hir::BinOpKind::BitXor => (
                                    format!("no implementation for `{} ^ {}`", lhs_ty, rhs_ty),
                                    Some("std::ops::BitXor"),
                                ),
                                hir::BinOpKind::BitOr => (
                                    format!("no implementation for `{} | {}`", lhs_ty, rhs_ty),
                                    Some("std::ops::BitOr"),
                                ),
                                hir::BinOpKind::Shl => (
                                    format!("no implementation for `{} << {}`", lhs_ty, rhs_ty),
                                    Some("std::ops::Shl"),
                                ),
                                hir::BinOpKind::Shr => (
                                    format!("no implementation for `{} >> {}`", lhs_ty, rhs_ty),
                                    Some("std::ops::Shr"),
                                ),
                                hir::BinOpKind::Eq | hir::BinOpKind::Ne => (
                                    format!(
                                        "binary operation `{}` cannot be applied to type `{}`",
                                        op.node.as_str(),
                                        lhs_ty
                                    ),
                                    Some("std::cmp::PartialEq"),
                                ),
                                hir::BinOpKind::Lt
                                | hir::BinOpKind::Le
                                | hir::BinOpKind::Gt
                                | hir::BinOpKind::Ge => (
                                    format!(
                                        "binary operation `{}` cannot be applied to type `{}`",
                                        op.node.as_str(),
                                        lhs_ty
                                    ),
                                    Some("std::cmp::PartialOrd"),
                                ),
                                _ => (
                                    format!(
                                        "binary operation `{}` cannot be applied to type `{}`",
                                        op.node.as_str(),
                                        lhs_ty
                                    ),
                                    None,
                                ),
                            };
                            let mut err = struct_span_err!(
                                self.tcx.sess,
                                op.span,
                                E0369,
                                "{}",
                                message.as_str()
                            );

                            let mut involves_fn = false;
                            if !lhs_expr.span.eq(&rhs_expr.span) {
                                involves_fn |= self.add_type_neq_err_label(
                                    &mut err,
                                    lhs_expr.span,
                                    lhs_ty,
                                    rhs_ty,
                                    op,
                                    is_assign,
                                );
                                involves_fn |= self.add_type_neq_err_label(
                                    &mut err,
                                    rhs_expr.span,
                                    rhs_ty,
                                    lhs_ty,
                                    op,
                                    is_assign,
                                );
                            }

                            let mut suggested_deref = false;
                            if let Ref(_, rty, _) = lhs_ty.kind {
                                if {
                                    self.infcx.type_is_copy_modulo_regions(
                                        self.param_env,
                                        rty,
                                        lhs_expr.span,
                                    ) && self
                                        .lookup_op_method(rty, &[rhs_ty], Op::Binary(op, is_assign))
                                        .is_ok()
                                } {
                                    if let Ok(lstring) = source_map.span_to_snippet(lhs_expr.span) {
                                        err.help(&format!(
                                            "`{}` can be used on '{}', you can \
                                            dereference `{2}`: `*{2}`",
                                            op.node.as_str(),
                                            rty.peel_refs(),
                                            lstring
                                        ));
                                        suggested_deref = true;
                                    }
                                }
                            }
                            if let Some(missing_trait) = missing_trait {
                                if op.node == hir::BinOpKind::Add
                                    && self.check_str_addition(
                                        lhs_expr, rhs_expr, lhs_ty, rhs_ty, &mut err, false, op,
                                    )
                                {
                                    // This has nothing here because it means we did string
                                    // concatenation (e.g., "Hello " + "World!"). This means
                                    // we don't want the note in the else clause to be emitted
                                } else if let ty::Param(_) = lhs_ty.kind {
                                    // FIXME: point to span of param
                                    err.note(&format!(
                                        "`{}` might need a bound for `{}`",
                                        lhs_ty, missing_trait
                                    ));
                                } else if !suggested_deref && !involves_fn {
                                    err.note(&format!(
                                        "an implementation of `{}` might \
                                         be missing for `{}`",
                                        missing_trait, lhs_ty
                                    ));
                                }
                            }
                            err.emit();
                        }
                    }
                }
                self.tcx.types.err
            }
        };

        (lhs_ty, rhs_ty, return_ty)
    }
