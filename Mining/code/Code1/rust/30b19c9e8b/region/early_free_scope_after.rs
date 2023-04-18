    pub fn early_free_scope<'a, 'gcx>(&self, tcx: TyCtxt<'a, 'gcx, 'tcx>,
                                       br: &ty::EarlyBoundRegion)
                                       -> Scope {
        let param_owner = tcx.parent_def_id(br.def_id).unwrap();

        let param_owner_id = tcx.hir.as_local_node_id(param_owner).unwrap();
        let scope = tcx.hir.maybe_body_owned_by(param_owner_id).map(|body_id| {
            tcx.hir.body(body_id).value.hir_id.local_id
        }).unwrap_or_else(|| {
            // The lifetime was defined on node that doesn't own a body,
            // which in practice can only mean a trait or an impl, that
            // is the parent of a method, and that is enforced below.
            assert_eq!(Some(param_owner_id), self.root_parent,
                       "free_scope: {:?} not recognized by the \
                        region scope tree for {:?} / {:?}",
                       param_owner,
                       self.root_parent.map(|id| tcx.hir.local_def_id(id)),
                       self.root_body.map(|hir_id| DefId::local(hir_id.owner)));

            // The trait/impl lifetime is in scope for the method's body.
            self.root_body.unwrap().local_id
        });

        Scope::CallSite(scope)
    }

    /// Assuming that the provided region was defined within this `ScopeTree`,
    /// returns the outermost `Scope` that the region outlives.
    pub fn free_scope<'a, 'gcx>(&self, tcx: TyCtxt<'a, 'gcx, 'tcx>, fr: &ty::FreeRegion)
                                 -> Scope {
        let param_owner = match fr.bound_region {
            ty::BoundRegion::BrNamed(def_id, _) => {
                tcx.parent_def_id(def_id).unwrap()
            }
            _ => fr.scope
        };

        // Ensure that the named late-bound lifetimes were defined
        // on the same function that they ended up being freed in.
        assert_eq!(param_owner, fr.scope);

        let param_owner_id = tcx.hir.as_local_node_id(param_owner).unwrap();
        let body_id = tcx.hir.body_owned_by(param_owner_id);
        Scope::CallSite(tcx.hir.body(body_id).value.hir_id.local_id)
    }

    /// Checks whether the given scope contains a `yield`. If so,
    /// returns `Some((span, expr_count))` with the span of a yield we found and
    /// the number of expressions appearing before the `yield` in the body.
    pub fn yield_in_scope(&self, scope: Scope) -> Option<(Span, usize)> {
        self.yield_in_scope.get(&scope).cloned()
    }

    /// Gives the number of expressions visited in a body.
    /// Used to sanity check visit_expr call count when
    /// calculating generator interiors.
    pub fn body_expr_count(&self, body_id: hir::BodyId) -> Option<usize> {
        self.body_expr_count.get(&body_id).map(|r| *r)
    }
}

/// Records the lifetime of a local variable as `cx.var_parent`
fn record_var_lifetime(visitor: &mut RegionResolutionVisitor,
                       var_id: hir::ItemLocalId,
                       _sp: Span) {
    match visitor.cx.var_parent {
        None => {
            // this can happen in extern fn declarations like
            //
            // extern fn isalnum(c: c_int) -> c_int
        }
        Some(parent_scope) =>
            visitor.scope_tree.record_var_scope(var_id, parent_scope),
    }
}

fn resolve_block<'a, 'tcx>(visitor: &mut RegionResolutionVisitor<'a, 'tcx>, blk: &'tcx hir::Block) {
    debug!("resolve_block(blk.id={:?})", blk.id);

    let prev_cx = visitor.cx;

    // We treat the tail expression in the block (if any) somewhat
    // differently from the statements. The issue has to do with
    // temporary lifetimes. Consider the following:
    //
    //    quux({
    //        let inner = ... (&bar()) ...;
    //
    //        (... (&foo()) ...) // (the tail expression)
    //    }, other_argument());
    //
    // Each of the statements within the block is a terminating
    // scope, and thus a temporary (e.g. the result of calling
    // `bar()` in the initalizer expression for `let inner = ...;`)
    // will be cleaned up immediately after its corresponding
    // statement (i.e. `let inner = ...;`) executes.
    //
    // On the other hand, temporaries associated with evaluating the
    // tail expression for the block are assigned lifetimes so that
    // they will be cleaned up as part of the terminating scope
    // *surrounding* the block expression. Here, the terminating
    // scope for the block expression is the `quux(..)` call; so
    // those temporaries will only be cleaned up *after* both
    // `other_argument()` has run and also the call to `quux(..)`
    // itself has returned.

    visitor.enter_node_scope_with_dtor(blk.hir_id.local_id);
    visitor.cx.var_parent = visitor.cx.parent;

    {
        // This block should be kept approximately in sync with
        // `intravisit::walk_block`. (We manually walk the block, rather
        // than call `walk_block`, in order to maintain precise
        // index information.)

        for (i, statement) in blk.stmts.iter().enumerate() {
            if let hir::StmtDecl(..) = statement.node {
                // Each StmtDecl introduces a subscope for bindings
                // introduced by the declaration; this subscope covers
                // a suffix of the block . Each subscope in a block
                // has the previous subscope in the block as a parent,
                // except for the first such subscope, which has the
                // block itself as a parent.
                visitor.enter_scope(
                    Scope::Remainder(BlockRemainder {
                        block: blk.hir_id.local_id,
                        first_statement_index: FirstStatementIndex::new(i)
                    })
                );
                visitor.cx.var_parent = visitor.cx.parent;
            }
            visitor.visit_stmt(statement)
        }
        walk_list!(visitor, visit_expr, &blk.expr);
    }

    visitor.cx = prev_cx;
}
