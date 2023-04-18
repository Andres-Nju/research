    fn normalize_associated_types_in<T>(&self,
                                        span: Span,
                                        body_id: hir::HirId,
                                        param_env: ty::ParamEnv<'tcx>,
                                        value: &T) -> T
        where T : TypeFoldable<'tcx>
    {
        let ok = self.partially_normalize_associated_types_in(span, body_id, param_env, value);
        self.register_infer_ok_obligations(ok)
    }
}

struct CheckItemTypesVisitor<'tcx> {
    tcx: TyCtxt<'tcx>,
}

impl ItemLikeVisitor<'tcx> for CheckItemTypesVisitor<'tcx> {
    fn visit_item(&mut self, i: &'tcx hir::Item) {
        check_item_type(self.tcx, i);
    }
    fn visit_trait_item(&mut self, _: &'tcx hir::TraitItem) { }
    fn visit_impl_item(&mut self, _: &'tcx hir::ImplItem) { }
}

pub fn check_wf_new(tcx: TyCtxt<'_>) {
    let mut visit = wfcheck::CheckTypeWellFormedVisitor::new(tcx);
    tcx.hir().krate().par_visit_all_item_likes(&mut visit);
}

fn check_mod_item_types(tcx: TyCtxt<'_>, module_def_id: DefId) {
    tcx.hir().visit_item_likes_in_module(module_def_id, &mut CheckItemTypesVisitor { tcx });
}

fn typeck_item_bodies(tcx: TyCtxt<'_>, crate_num: CrateNum) {
    debug_assert!(crate_num == LOCAL_CRATE);
    tcx.par_body_owners(|body_owner_def_id| {
        tcx.ensure().typeck_tables_of(body_owner_def_id);
    });
}

fn check_item_well_formed(tcx: TyCtxt<'_>, def_id: DefId) {
    wfcheck::check_item_well_formed(tcx, def_id);
}

fn check_trait_item_well_formed(tcx: TyCtxt<'_>, def_id: DefId) {
    wfcheck::check_trait_item(tcx, def_id);
}

fn check_impl_item_well_formed(tcx: TyCtxt<'_>, def_id: DefId) {
    wfcheck::check_impl_item(tcx, def_id);
}

pub fn provide(providers: &mut Providers<'_>) {
    method::provide(providers);
    *providers = Providers {
        typeck_item_bodies,
        typeck_tables_of,
        has_typeck_tables,
        adt_destructor,
        used_trait_imports,
        check_item_well_formed,
        check_trait_item_well_formed,
        check_impl_item_well_formed,
        check_mod_item_types,
        ..*providers
    };
}

fn adt_destructor(tcx: TyCtxt<'_>, def_id: DefId) -> Option<ty::Destructor> {
    tcx.calculate_dtor(def_id, &mut dropck::check_drop_impl)
}

/// If this `DefId` is a "primary tables entry", returns
/// `Some((body_id, header, decl))` with information about
/// it's body-id, fn-header and fn-decl (if any). Otherwise,
/// returns `None`.
///
/// If this function returns `Some`, then `typeck_tables(def_id)` will
/// succeed; if it returns `None`, then `typeck_tables(def_id)` may or
/// may not succeed. In some cases where this function returns `None`
/// (notably closures), `typeck_tables(def_id)` would wind up
/// redirecting to the owning function.
fn primary_body_of(
    tcx: TyCtxt<'_>,
    id: hir::HirId,
) -> Option<(hir::BodyId, Option<&hir::Ty>, Option<&hir::FnHeader>, Option<&hir::FnDecl>)> {
    match tcx.hir().get(id) {
        Node::Item(item) => {
            match item.kind {
                hir::ItemKind::Const(ref ty, body) |
                hir::ItemKind::Static(ref ty, _, body) =>
                    Some((body, Some(ty), None, None)),
                hir::ItemKind::Fn(ref sig, .., body) =>
                    Some((body, None, Some(&sig.header), Some(&sig.decl))),
                _ =>
                    None,
            }
        }
        Node::TraitItem(item) => {
            match item.kind {
                hir::TraitItemKind::Const(ref ty, Some(body)) =>
                    Some((body, Some(ty), None, None)),
                hir::TraitItemKind::Method(ref sig, hir::TraitMethod::Provided(body)) =>
                    Some((body, None, Some(&sig.header), Some(&sig.decl))),
                _ =>
                    None,
            }
        }
        Node::ImplItem(item) => {
            match item.kind {
                hir::ImplItemKind::Const(ref ty, body) =>
                    Some((body, Some(ty), None, None)),
                hir::ImplItemKind::Method(ref sig, body) =>
                    Some((body, None, Some(&sig.header), Some(&sig.decl))),
                _ =>
                    None,
            }
        }
        Node::AnonConst(constant) => Some((constant.body, None, None, None)),
        _ => None,
    }
}

fn has_typeck_tables(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    // Closures' tables come from their outermost function,
    // as they are part of the same "inference environment".
    let outer_def_id = tcx.closure_base_def_id(def_id);
    if outer_def_id != def_id {
        return tcx.has_typeck_tables(outer_def_id);
    }

    let id = tcx.hir().as_local_hir_id(def_id).unwrap();
    primary_body_of(tcx, id).is_some()
}

fn used_trait_imports(tcx: TyCtxt<'_>, def_id: DefId) -> &DefIdSet {
    &*tcx.typeck_tables_of(def_id).used_trait_imports
}

/// Inspects the substs of opaque types, replacing any inference variables
/// with proper generic parameter from the identity substs.
///
/// This is run after we normalize the function signature, to fix any inference
/// variables introduced by the projection of associated types. This ensures that
/// any opaque types used in the signature continue to refer to generic parameters,
/// allowing them to be considered for defining uses in the function body
///
/// For example, consider this code.
///
/// ```rust
/// trait MyTrait {
///     type MyItem;
///     fn use_it(self) -> Self::MyItem
/// }
/// impl<T, I> MyTrait for T where T: Iterator<Item = I> {
///     type MyItem = impl Iterator<Item = I>;
///     fn use_it(self) -> Self::MyItem {
///         self
///     }
/// }
/// ```
///
/// When we normalize the signature of `use_it` from the impl block,
/// we will normalize `Self::MyItem` to the opaque type `impl Iterator<Item = I>`
/// However, this projection result may contain inference variables, due
/// to the way that projection works. We didn't have any inference variables
/// in the signature to begin with - leaving them in will cause us to incorrectly
/// conclude that we don't have a defining use of `MyItem`. By mapping inference
/// variables back to the actual generic parameters, we will correctly see that
/// we have a defining use of `MyItem`
fn fixup_opaque_types<'tcx, T>(tcx: TyCtxt<'tcx>, val: &T) -> T where T: TypeFoldable<'tcx> {
    struct FixupFolder<'tcx> {
        tcx: TyCtxt<'tcx>
    }

    impl<'tcx> TypeFolder<'tcx> for FixupFolder<'tcx> {
        fn tcx<'a>(&'a self) -> TyCtxt<'tcx> {
            self.tcx
        }

        fn fold_ty(&mut self, ty: Ty<'tcx>) -> Ty<'tcx> {
            match ty.kind {
                ty::Opaque(def_id, substs) => {
                    debug!("fixup_opaque_types: found type {:?}", ty);
                    // Here, we replace any inference variables that occur within
                    // the substs of an opaque type. By definition, any type occuring
                    // in the substs has a corresponding generic parameter, which is what
                    // we replace it with.
                    // This replacement is only run on the function signature, so any
                    // inference variables that we come across must be the rust of projection
                    // (there's no other way for a user to get inference variables into
                    // a function signature).
                    if ty.needs_infer() {
                        let new_substs = InternalSubsts::for_item(self.tcx, def_id, |param, _| {
                            let old_param = substs[param.index as usize];
                            match old_param.unpack() {
                                GenericArgKind::Type(old_ty) => {
                                    if let ty::Infer(_) = old_ty.kind {
                                        // Replace inference type with a generic parameter
                                        self.tcx.mk_param_from_def(param)
                                    } else {
                                        old_param.fold_with(self)
                                    }
                                },
                                GenericArgKind::Const(old_const) => {
                                    if let ConstValue::Infer(_) = old_const.val {
                        // This should never happen - we currently do not support
                        // 'const projections', e.g.:
                        // `impl<T: SomeTrait> MyTrait for T where <T as SomeTrait>::MyConst == 25`
                        // which should be the only way for us to end up with a const inference
                        // variable after projection. If Rust ever gains support for this kind
                        // of projection, this should *probably* be changed to
                        // `self.tcx.mk_param_from_def(param)`
                                        bug!("Found infer const: `{:?}` in opaque type: {:?}",
                                             old_const, ty);
                                    } else {
                                        old_param.fold_with(self)
                                    }
                                }
                                GenericArgKind::Lifetime(old_region) => {
                                    if let RegionKind::ReVar(_) = old_region {
                                        self.tcx.mk_param_from_def(param)
                                    } else {
                                        old_param.fold_with(self)
                                    }
                                }
                            }
                        });
                        let new_ty = self.tcx.mk_opaque(def_id, new_substs);
                        debug!("fixup_opaque_types: new type: {:?}", new_ty);
                        new_ty
                    } else {
