    fn visit_place(&mut self,
                    place: &Place<'tcx>,
                    context: PlaceContext<'tcx>,
                    location: Location) {
        if let PlaceContext::Borrow { .. } = context {
            if util::is_disaligned(self.tcx, self.mir, self.param_env, place) {
                let source_info = self.source_info;
                let lint_root =
                    self.source_scope_local_data[source_info.scope].lint_root;
                self.register_violations(&[UnsafetyViolation {
                    source_info,
                    description: Symbol::intern("borrow of packed field").as_interned_str(),
                    details:
                        Symbol::intern("fields of packed structs might be misaligned: \
                                        dereferencing a misaligned pointer or even just creating a \
                                        misaligned reference is undefined behavior")
                            .as_interned_str(),
                    kind: UnsafetyViolationKind::BorrowPacked(lint_root)
                }], &[]);
            }
        }

        match place {
            &Place::Projection(box Projection {
                ref base, ref elem
            }) => {
                let old_source_info = self.source_info;
                if let &Place::Local(local) = base {
                    if self.mir.local_decls[local].internal {
                        // Internal locals are used in the `move_val_init` desugaring.
                        // We want to check unsafety against the source info of the
                        // desugaring, rather than the source info of the RHS.
                        self.source_info = self.mir.local_decls[local].source_info;
                    }
                }
                let base_ty = base.ty(self.mir, self.tcx).to_ty(self.tcx);
                match base_ty.sty {
                    ty::TyRawPtr(..) => {
                        self.require_unsafe("dereference of raw pointer",
                            "raw pointers may be NULL, dangling or unaligned; they can violate \
                             aliasing rules and cause data races: all of these are undefined \
                             behavior")
                    }
                    ty::TyAdt(adt, _) => {
                        if adt.is_union() {
                            if context == PlaceContext::Store ||
                                context == PlaceContext::AsmOutput ||
                                context == PlaceContext::Drop
                            {
                                let elem_ty = match elem {
                                    &ProjectionElem::Field(_, ty) => ty,
                                    _ => span_bug!(
                                        self.source_info.span,
                                        "non-field projection {:?} from union?",
                                        place)
                                };
                                if elem_ty.moves_by_default(self.tcx, self.param_env,
                                                            self.source_info.span) {
                                    self.require_unsafe(
                                        "assignment to non-`Copy` union field",
                                        "the previous content of the field may be dropped, which \
                                         cause undefined behavior if the field was not properly \
                                         initialized")
                                } else {
                                    // write to non-move union, safe
                                }
                            } else {
                                self.require_unsafe("access to union field",
                                    "the field may not be properly initialized: using \
                                     uninitialized data will cause undefined behavior")
                            }
                        }
                    }
                    _ => {}
                }
                self.source_info = old_source_info;
            }
            &Place::Local(..) => {
                // locals are safe
            }
            &Place::Static(box Static { def_id, ty: _ }) => {
                if self.tcx.is_static(def_id) == Some(hir::Mutability::MutMutable) {
                    self.require_unsafe("use of mutable static",
                        "mutable statics can be mutated by multiple threads: aliasing violations \
                         or data races will cause undefined behavior");
                } else if self.tcx.is_foreign_item(def_id) {
                    let source_info = self.source_info;
                    let lint_root =
                        self.source_scope_local_data[source_info.scope].lint_root;
                    self.register_violations(&[UnsafetyViolation {
                        source_info,
                        description: Symbol::intern("use of extern static").as_interned_str(),
                        details:
                            Symbol::intern("extern statics are not controlled by the Rust type \
                                            system: invalid data, aliasing violations or data \
                                            races will cause undefined behavior")
                                .as_interned_str(),
                        kind: UnsafetyViolationKind::ExternStatic(lint_root)
                    }], &[]);
                }
            }
        };
        self.super_place(place, context, location);
    }
