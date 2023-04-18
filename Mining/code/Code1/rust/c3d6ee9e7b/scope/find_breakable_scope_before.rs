    pub fn find_breakable_scope(&mut self,
                           span: Span,
                           label: region::Scope)
                           -> &mut BreakableScope<'tcx> {
        // find the loop-scope with the correct id
        self.breakable_scopes.iter_mut()
            .rev()
            .filter(|breakable_scope| breakable_scope.region_scope == label)
            .next()
            .unwrap_or_else(|| span_bug!(span, "no enclosing breakable scope found"))
    }

    /// Given a span and the current source scope, make a SourceInfo.
    pub fn source_info(&self, span: Span) -> SourceInfo {
        SourceInfo {
            span,
            scope: self.source_scope
        }
    }

    /// Returns the `region::Scope` of the scope which should be exited by a
    /// return.
    pub fn region_scope_of_return_scope(&self) -> region::Scope {
        // The outermost scope (`scopes[0]`) will be the `CallSiteScope`.
        // We want `scopes[1]`, which is the `ParameterScope`.
        assert!(self.scopes.len() >= 2);
        assert!(match self.scopes[1].region_scope.data() {
            region::ScopeData::Arguments(_) => true,
            _ => false,
        });
        self.scopes[1].region_scope
    }

    /// Returns the topmost active scope, which is known to be alive until
    /// the next scope expression.
    pub fn topmost_scope(&self) -> region::Scope {
        self.scopes.last().expect("topmost_scope: no scopes present").region_scope
    }

    /// Returns the scope that we should use as the lifetime of an
    /// operand. Basically, an operand must live until it is consumed.
    /// This is similar to, but not quite the same as, the temporary
    /// scope (which can be larger or smaller).
    ///
    /// Consider:
    ///
    ///     let x = foo(bar(X, Y));
    ///
    /// We wish to pop the storage for X and Y after `bar()` is
    /// called, not after the whole `let` is completed.
    ///
    /// As another example, if the second argument diverges:
    ///
    ///     foo(Box::new(2), panic!())
    ///
    /// We would allocate the box but then free it on the unwinding
    /// path; we would also emit a free on the 'success' path from
    /// panic, but that will turn out to be removed as dead-code.
    ///
    /// When building statics/constants, returns `None` since
    /// intermediate values do not have to be dropped in that case.
    pub fn local_scope(&self) -> Option<region::Scope> {
        match self.hir.body_owner_kind {
            hir::BodyOwnerKind::Const |
            hir::BodyOwnerKind::Static(_) =>
                // No need to free storage in this context.
                None,
            hir::BodyOwnerKind::Fn =>
                Some(self.topmost_scope()),
        }
    }

    // Schedule an abort block - this is used for some ABIs that cannot unwind
    pub fn schedule_abort(&mut self) -> BasicBlock {
        self.scopes[0].needs_cleanup = true;
        let abortblk = self.cfg.start_new_cleanup_block();
        let source_info = self.scopes[0].source_info(self.fn_span);
        self.cfg.terminate(abortblk, source_info, TerminatorKind::Abort);
        self.cached_resume_block = Some(abortblk);
        abortblk
    }

    // Scheduling drops
    // ================
    /// Indicates that `place` should be dropped on exit from
    /// `region_scope`.
    pub fn schedule_drop(&mut self,
                         span: Span,
                         region_scope: region::Scope,
                         place: &Place<'tcx>,
                         place_ty: Ty<'tcx>) {
        let needs_drop = self.hir.needs_drop(place_ty);
        let drop_kind = if needs_drop {
            DropKind::Value { cached_block: CachedBlock::default() }
        } else {
            // Only temps and vars need their storage dead.
            match *place {
                Place::Local(index) if index.index() > self.arg_count => DropKind::Storage,
                _ => return
            }
        };

        for scope in self.scopes.iter_mut().rev() {
            let this_scope = scope.region_scope == region_scope;
            // When building drops, we try to cache chains of drops in such a way so these drops
            // could be reused by the drops which would branch into the cached (already built)
            // blocks.  This, however, means that whenever we add a drop into a scope which already
            // had some blocks built (and thus, cached) for it, we must invalidate all caches which
            // might branch into the scope which had a drop just added to it. This is necessary,
            // because otherwise some other code might use the cache to branch into already built
            // chain of drops, essentially ignoring the newly added drop.
            //
            // For example consider there’s two scopes with a drop in each. These are built and
            // thus the caches are filled:
            //
            // +--------------------------------------------------------+
            // | +---------------------------------+                    |
            // | | +--------+     +-------------+  |  +---------------+ |
            // | | | return | <-+ | drop(outer) | <-+ |  drop(middle) | |
            // | | +--------+     +-------------+  |  +---------------+ |
            // | +------------|outer_scope cache|--+                    |
            // +------------------------------|middle_scope cache|------+
            //
            // Now, a new, inner-most scope is added along with a new drop into both inner-most and
            // outer-most scopes:
            //
            // +------------------------------------------------------------+
            // | +----------------------------------+                       |
            // | | +--------+      +-------------+  |   +---------------+   | +-------------+
            // | | | return | <+   | drop(new)   | <-+  |  drop(middle) | <--+| drop(inner) |
            // | | +--------+  |   | drop(outer) |  |   +---------------+   | +-------------+
            // | |             +-+ +-------------+  |                       |
            // | +---|invalid outer_scope cache|----+                       |
            // +----=----------------|invalid middle_scope cache|-----------+
            //
            // If, when adding `drop(new)` we do not invalidate the cached blocks for both
            // outer_scope and middle_scope, then, when building drops for the inner (right-most)
            // scope, the old, cached blocks, without `drop(new)` will get used, producing the
            // wrong results.
            //
            // The cache and its invalidation for unwind branch is somewhat special. The cache is
            // per-drop, rather than per scope, which has a several different implications. Adding
            // a new drop into a scope will not invalidate cached blocks of the prior drops in the
            // scope. That is true, because none of the already existing drops will have an edge
            // into a block with the newly added drop.
            //
            // Note that this code iterates scopes from the inner-most to the outer-most,
            // invalidating caches of each scope visited. This way bare minimum of the
            // caches gets invalidated. i.e. if a new drop is added into the middle scope, the
            // cache of outer scpoe stays intact.
            scope.invalidate_cache(!needs_drop, this_scope);
            if this_scope {
                if let DropKind::Value { .. } = drop_kind {
                    scope.needs_cleanup = true;
                }

                let region_scope_span = region_scope.span(self.hir.tcx(),
                                                          &self.hir.region_scope_tree);
                // Attribute scope exit drops to scope's closing brace.
                let scope_end = self.hir.tcx().sess.codemap().end_point(region_scope_span);

                scope.drops.push(DropData {
                    span: scope_end,
                    location: place.clone(),
                    kind: drop_kind
                });
