pub fn resolve_interior<'a, 'tcx>(
    fcx: &'a FnCtxt<'a, 'tcx>,
    def_id: DefId,
    body_id: hir::BodyId,
    interior: Ty<'tcx>,
    kind: hir::GeneratorKind,
) {
    let body = fcx.tcx.hir().body(body_id);
    let mut visitor = InteriorVisitor {
        fcx,
        types: FxHashMap::default(),
        region_scope_tree: fcx.tcx.region_scope_tree(def_id),
        expr_count: 0,
        kind,
    };
    intravisit::walk_body(&mut visitor, body);

    // Check that we visited the same amount of expressions and the RegionResolutionVisitor
    let region_expr_count = visitor.region_scope_tree.body_expr_count(body_id).unwrap();
    assert_eq!(region_expr_count, visitor.expr_count);

    let mut types: Vec<_> = visitor.types.drain().collect();

    // Sort types by insertion order
    types.sort_by_key(|t| t.1);

    // The types in the generator interior contain lifetimes local to the generator itself,
    // which should not be exposed outside of the generator. Therefore, we replace these
    // lifetimes with existentially-bound lifetimes, which reflect the exact value of the
    // lifetimes not being known by users.
    //
    // These lifetimes are used in auto trait impl checking (for example,
    // if a Sync generator contains an &'α T, we need to check whether &'α T: Sync),
    // so knowledge of the exact relationships between them isn't particularly important.

    debug!(
        "types in generator {:?}, span = {:?}",
        types.iter().map(|t| (t.0).ty).collect::<Vec<_>>(),
        body.value.span,
    );

    // Replace all regions inside the generator interior with late bound regions
    // Note that each region slot in the types gets a new fresh late bound region,
    // which means that none of the regions inside relate to any other, even if
    // typeck had previously found constraints that would cause them to be related.
    let mut counter = 0;
    let types = fcx.tcx.fold_regions(&types, &mut false, |_, current_depth| {
        counter += 1;
        fcx.tcx.mk_region(ty::ReLateBound(current_depth, ty::BrAnon(counter)))
    });

    // Store the generator types and spans into the tables for this generator.
    let interior_types = types.iter().map(|t| t.0.clone()).collect::<Vec<_>>();
    visitor.fcx.inh.tables.borrow_mut().generator_interior_types = interior_types;

    // Extract type components
    let type_list = fcx.tcx.mk_type_list(types.into_iter().map(|t| (t.0).ty));

    let witness = fcx.tcx.mk_generator_witness(ty::Binder::bind(type_list));

    debug!("types in generator after region replacement {:?}, span = {:?}",
            witness, body.value.span);

    // Unify the type variable inside the generator with the new witness
    match fcx.at(&fcx.misc(body.value.span), fcx.param_env).eq(interior, witness) {
        Ok(ok) => fcx.register_infer_ok_obligations(ok),
        _ => bug!(),
    }
}
