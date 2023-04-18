    fn get_base_type_def_id(&self, span: Span, ty: Ty<'tcx>) -> Option<DefId> {
        match ty.sty {
            TyEnum(def, _) |
            TyStruct(def, _) => {
                Some(def.did)
            }

            TyTrait(ref t) => {
                Some(t.principal.def_id())
            }

            TyBox(_) => {
                self.inference_context.tcx.lang_items.owned_box()
            }

            TyBool | TyChar | TyInt(..) | TyUint(..) | TyFloat(..) |
            TyStr | TyArray(..) | TySlice(..) | TyFnDef(..) | TyFnPtr(_) |
            TyTuple(..) | TyParam(..) | TyError | TyNever |
            TyRawPtr(_) | TyRef(_, _) | TyProjection(..) => {
                None
            }

            TyInfer(..) | TyClosure(..) | TyAnon(..) => {
                // `ty` comes from a user declaration so we should only expect types
                // that the user can type
                span_bug!(
                    span,
                    "coherence encountered unexpected type searching for base type: {}",
                    ty);
            }
        }
    }

    fn check(&self) {
        // Check implementations and traits. This populates the tables
        // containing the inherent methods and extension methods. It also
        // builds up the trait inheritance table.
        self.crate_context.tcx.visit_all_items_in_krate(
            DepNode::CoherenceCheckImpl,
            &mut CoherenceCheckVisitor { cc: self });

        // Copy over the inherent impls we gathered up during the walk into
        // the tcx.
        let mut tcx_inherent_impls =
            self.crate_context.tcx.inherent_impls.borrow_mut();
        for (k, v) in self.inherent_impls.borrow().iter() {
            tcx_inherent_impls.insert((*k).clone(),
                                      Rc::new((*v.borrow()).clone()));
        }

        // Populate the table of destructors. It might seem a bit strange to
        // do this here, but it's actually the most convenient place, since
        // the coherence tables contain the trait -> type mappings.
        self.populate_destructors();

        // Check to make sure implementations of `Copy` are legal.
        self.check_implementations_of_copy();

        // Check to make sure implementations of `CoerceUnsized` are legal
        // and collect the necessary information from them.
        self.check_implementations_of_coerce_unsized();
    }

    fn check_implementation(&self, item: &Item) {
        let tcx = self.crate_context.tcx;
        let impl_did = tcx.map.local_def_id(item.id);
        let self_type = tcx.lookup_item_type(impl_did);

        // If there are no traits, then this implementation must have a
        // base type.

        let impl_items = self.create_impl_from_item(item);

        if let Some(trait_ref) = self.crate_context.tcx.impl_trait_ref(impl_did) {
            debug!("(checking implementation) adding impl for trait '{:?}', item '{}'",
                   trait_ref,
                   item.name);

            // Skip impls where one of the self type is an error type.
            // This occurs with e.g. resolve failures (#30589).
            if trait_ref.references_error() {
                return;
            }

            enforce_trait_manually_implementable(self.crate_context.tcx,
                                                 item.span,
                                                 trait_ref.def_id);
            self.add_trait_impl(trait_ref, impl_did);
        } else {
            // Skip inherent impls where the self type is an error
            // type. This occurs with e.g. resolve failures (#30589).
            if self_type.ty.references_error() {
                return;
            }

            // Add the implementation to the mapping from implementation to base
            // type def ID, if there is a base type for this implementation and
            // the implementation does not have any associated traits.
            if let Some(base_def_id) = self.get_base_type_def_id(item.span, self_type.ty) {
                self.add_inherent_impl(base_def_id, impl_did);
            }
        }

        tcx.impl_items.borrow_mut().insert(impl_did, impl_items);
    }

    fn add_inherent_impl(&self, base_def_id: DefId, impl_def_id: DefId) {
        if let Some(implementation_list) = self.inherent_impls.borrow().get(&base_def_id) {
            implementation_list.borrow_mut().push(impl_def_id);
            return;
        }

        self.inherent_impls.borrow_mut().insert(
            base_def_id,
            Rc::new(RefCell::new(vec!(impl_def_id))));
    }

    fn add_trait_impl(&self, impl_trait_ref: ty::TraitRef<'gcx>, impl_def_id: DefId) {
        debug!("add_trait_impl: impl_trait_ref={:?} impl_def_id={:?}",
               impl_trait_ref, impl_def_id);
        let trait_def = self.crate_context.tcx.lookup_trait_def(impl_trait_ref.def_id);
        trait_def.record_local_impl(self.crate_context.tcx, impl_def_id, impl_trait_ref);
    }

    // Converts an implementation in the AST to a vector of items.
    fn create_impl_from_item(&self, item: &Item) -> Vec<ImplOrTraitItemId> {
        match item.node {
            ItemImpl(_, _, _, _, _, ref impl_items) => {
                impl_items.iter().map(|impl_item| {
                    let impl_def_id = self.crate_context.tcx.map.local_def_id(impl_item.id);
                    match impl_item.node {
                        hir::ImplItemKind::Const(..) => {
                            ConstTraitItemId(impl_def_id)
                        }
                        hir::ImplItemKind::Method(..) => {
                            MethodTraitItemId(impl_def_id)
                        }
                        hir::ImplItemKind::Type(_) => {
                            TypeTraitItemId(impl_def_id)
                        }
                    }
                }).collect()
            }
            _ => {
                span_bug!(item.span, "can't convert a non-impl to an impl");
            }
        }
    }

    //
    // Destructors
    //

    fn populate_destructors(&self) {
        let tcx = self.crate_context.tcx;
        let drop_trait = match tcx.lang_items.drop_trait() {
            Some(id) => id, None => { return }
        };
        tcx.populate_implementations_for_trait_if_necessary(drop_trait);
        let drop_trait = tcx.lookup_trait_def(drop_trait);

        let impl_items = tcx.impl_items.borrow();

        drop_trait.for_each_impl(tcx, |impl_did| {
            let items = impl_items.get(&impl_did).unwrap();
            if items.is_empty() {
                // We'll error out later. For now, just don't ICE.
                return;
            }
            let method_def_id = items[0];

            let self_type = tcx.lookup_item_type(impl_did);
            match self_type.ty.sty {
                ty::TyEnum(type_def, _) |
                ty::TyStruct(type_def, _) => {
                    type_def.set_destructor(method_def_id.def_id());
                }
                _ => {
                    // Destructors only work on nominal types.
                    if let Some(impl_node_id) = tcx.map.as_local_node_id(impl_did) {
                        match tcx.map.find(impl_node_id) {
                            Some(hir_map::NodeItem(item)) => {
                                let span = match item.node {
                                    ItemImpl(_, _, _, _, ref ty, _) => {
                                        ty.span
                                    },
                                    _ => item.span
                                };
                                struct_span_err!(tcx.sess, span, E0120,
                                    "the Drop trait may only be implemented on structures")
                                    .span_label(span,
                                                &format!("implementing Drop requires a struct"))
                                    .emit();
                            }
                            _ => {
                                bug!("didn't find impl in ast map");
                            }
                        }
                    } else {
                        bug!("found external impl of Drop trait on \
                              something other than a struct");
                    }
                }
            }
        });
    }

    /// Ensures that implementations of the built-in trait `Copy` are legal.
    fn check_implementations_of_copy(&self) {
        let tcx = self.crate_context.tcx;
        let copy_trait = match tcx.lang_items.copy_trait() {
            Some(id) => id,
            None => return,
        };
        tcx.populate_implementations_for_trait_if_necessary(copy_trait);
        let copy_trait = tcx.lookup_trait_def(copy_trait);

        copy_trait.for_each_impl(tcx, |impl_did| {
            debug!("check_implementations_of_copy: impl_did={:?}",
                   impl_did);

            let impl_node_id = if let Some(n) = tcx.map.as_local_node_id(impl_did) {
                n
            } else {
                debug!("check_implementations_of_copy(): impl not in this \
                        crate");
                return
            };

            let self_type = tcx.lookup_item_type(impl_did);
            debug!("check_implementations_of_copy: self_type={:?} (bound)",
                   self_type);

            let span = tcx.map.span(impl_node_id);
            let param_env = ParameterEnvironment::for_item(tcx, impl_node_id);
            let self_type = self_type.ty.subst(tcx, &param_env.free_substs);
            assert!(!self_type.has_escaping_regions());

            debug!("check_implementations_of_copy: self_type={:?} (free)",
                   self_type);

            match param_env.can_type_implement_copy(tcx, self_type, span) {
                Ok(()) => {}
                Err(CopyImplementationError::InfrigingField(name)) => {
                       struct_span_err!(tcx.sess, span, E0204,
                                 "the trait `Copy` may not be implemented for \
                                 this type")
                           .span_label(span, &format!(
                                 "field `{}` does not implement `Copy`", name)
                               )
                           .emit()

                }
                Err(CopyImplementationError::InfrigingVariant(name)) => {
                    let item = tcx.map.expect_item(impl_node_id);
                    let span = if let ItemImpl(_, _, _, Some(ref tr), _, _) = item.node {
                        tr.path.span
                    } else {
                        span
                    };

                    struct_span_err!(tcx.sess, span, E0205,
                                     "the trait `Copy` may not be implemented for this type")
                        .span_label(span, &format!("variant `{}` does not implement `Copy`",
                                                   name))
                        .emit()
                }
                Err(CopyImplementationError::NotAnAdt) => {
                    let item = tcx.map.expect_item(impl_node_id);
                    let span = if let ItemImpl(_, _, _, _, ref ty, _) = item.node {
                        ty.span
                    } else {
                        span
                    };

                    struct_span_err!(tcx.sess, span, E0206,
                                     "the trait `Copy` may not be implemented for this type")
                        .span_label(span, &format!("type is not a structure or enumeration"))
                        .emit();
                }
                Err(CopyImplementationError::HasDestructor) => {
                    span_err!(tcx.sess, span, E0184,
                              "the trait `Copy` may not be implemented for this type; \
                               the type has a destructor");
                }
            }
        });
    }

    /// Process implementations of the built-in trait `CoerceUnsized`.
    fn check_implementations_of_coerce_unsized(&self) {
        let tcx = self.crate_context.tcx;
        let coerce_unsized_trait = match tcx.lang_items.coerce_unsized_trait() {
            Some(id) => id,
            None => return,
        };
        let unsize_trait = match tcx.lang_items.require(UnsizeTraitLangItem) {
            Ok(id) => id,
            Err(err) => {
                tcx.sess.fatal(&format!("`CoerceUnsized` implementation {}", err));
            }
        };

        let trait_def = tcx.lookup_trait_def(coerce_unsized_trait);

        trait_def.for_each_impl(tcx, |impl_did| {
            debug!("check_implementations_of_coerce_unsized: impl_did={:?}",
                   impl_did);

            let impl_node_id = if let Some(n) = tcx.map.as_local_node_id(impl_did) {
                n
            } else {
                debug!("check_implementations_of_coerce_unsized(): impl not \
                        in this crate");
                return;
            };

            let source = tcx.lookup_item_type(impl_did).ty;
            let trait_ref = self.crate_context.tcx.impl_trait_ref(impl_did).unwrap();
            let target = trait_ref.substs.type_at(1);
            debug!("check_implementations_of_coerce_unsized: {:?} -> {:?} (bound)",
                   source, target);

            let span = tcx.map.span(impl_node_id);
            let param_env = ParameterEnvironment::for_item(tcx, impl_node_id);
            let source = source.subst(tcx, &param_env.free_substs);
            let target = target.subst(tcx, &param_env.free_substs);
            assert!(!source.has_escaping_regions());

            debug!("check_implementations_of_coerce_unsized: {:?} -> {:?} (free)",
                   source, target);

            tcx.infer_ctxt(None, Some(param_env), Reveal::ExactMatch).enter(|infcx| {
                let origin = TypeOrigin::Misc(span);
                let check_mutbl = |mt_a: ty::TypeAndMut<'gcx>, mt_b: ty::TypeAndMut<'gcx>,
                                   mk_ptr: &Fn(Ty<'gcx>) -> Ty<'gcx>| {
                    if (mt_a.mutbl, mt_b.mutbl) == (hir::MutImmutable, hir::MutMutable) {
                        infcx.report_mismatched_types(origin, mk_ptr(mt_b.ty),
                                                      target, ty::error::TypeError::Mutability);
                    }
                    (mt_a.ty, mt_b.ty, unsize_trait, None)
                };
                let (source, target, trait_def_id, kind) = match (&source.sty, &target.sty) {
                    (&ty::TyBox(a), &ty::TyBox(b)) => (a, b, unsize_trait, None),

                    (&ty::TyRef(r_a, mt_a), &ty::TyRef(r_b, mt_b)) => {
                        infcx.sub_regions(infer::RelateObjectBound(span), r_b, r_a);
                        check_mutbl(mt_a, mt_b, &|ty| tcx.mk_imm_ref(r_b, ty))
                    }

                    (&ty::TyRef(_, mt_a), &ty::TyRawPtr(mt_b)) |
                    (&ty::TyRawPtr(mt_a), &ty::TyRawPtr(mt_b)) => {
                        check_mutbl(mt_a, mt_b, &|ty| tcx.mk_imm_ptr(ty))
                    }

                    (&ty::TyStruct(def_a, substs_a), &ty::TyStruct(def_b, substs_b)) => {
                        if def_a != def_b {
                            let source_path = tcx.item_path_str(def_a.did);
                            let target_path = tcx.item_path_str(def_b.did);
                            span_err!(tcx.sess, span, E0377,
                                      "the trait `CoerceUnsized` may only be implemented \
                                       for a coercion between structures with the same \
                                       definition; expected {}, found {}",
                                      source_path, target_path);
                            return;
                        }

                        let fields = &def_a.struct_variant().fields;
                        let diff_fields = fields.iter().enumerate().filter_map(|(i, f)| {
                            let (a, b) = (f.ty(tcx, substs_a), f.ty(tcx, substs_b));

                            if f.unsubst_ty().is_phantom_data() {
                                // Ignore PhantomData fields
                                None
                            } else if infcx.sub_types(false, origin, b, a).is_ok() {
                                // Ignore fields that aren't significantly changed
                                None
                            } else {
                                // Collect up all fields that were significantly changed
                                // i.e. those that contain T in coerce_unsized T -> U
                                Some((i, a, b))
                            }
                        }).collect::<Vec<_>>();

                        if diff_fields.is_empty() {
                            span_err!(tcx.sess, span, E0374,
                                      "the trait `CoerceUnsized` may only be implemented \
                                       for a coercion between structures with one field \
                                       being coerced, none found");
                            return;
                        } else if diff_fields.len() > 1 {
