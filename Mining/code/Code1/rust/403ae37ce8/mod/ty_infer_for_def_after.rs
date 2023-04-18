    fn ty_infer_for_def(&self,
                        ty_param_def: &ty::TypeParameterDef,
                        substs: &[Kind<'tcx>],
                        span: Span) -> Ty<'tcx> {
        self.type_var_for_def(span, ty_param_def, substs)
    }

    fn projected_ty_from_poly_trait_ref(&self,
                                        span: Span,
                                        poly_trait_ref: ty::PolyTraitRef<'tcx>,
                                        item_name: ast::Name)
                                        -> Ty<'tcx>
    {
        let (trait_ref, _) =
            self.replace_late_bound_regions_with_fresh_var(
                span,
                infer::LateBoundRegionConversionTime::AssocTypeProjection(item_name),
                &poly_trait_ref);

        self.tcx().mk_projection(trait_ref, item_name)
    }

    fn normalize_ty(&self, span: Span, ty: Ty<'tcx>) -> Ty<'tcx> {
        if ty.has_escaping_regions() {
            ty // FIXME: normalization and escaping regions
        } else {
            self.normalize_associated_types_in(span, &ty)
        }
    }

    fn set_tainted_by_errors(&self) {
        self.infcx.set_tainted_by_errors()
    }
}

/// Controls whether the arguments are tupled. This is used for the call
/// operator.
///
/// Tupling means that all call-side arguments are packed into a tuple and
/// passed as a single parameter. For example, if tupling is enabled, this
/// function:
///
///     fn f(x: (isize, isize))
///
/// Can be called as:
///
///     f(1, 2);
///
/// Instead of:
///
///     f((1, 2));
#[derive(Clone, Eq, PartialEq)]
enum TupleArgumentsFlag {
    DontTupleArguments,
    TupleArguments,
}

impl<'a, 'gcx, 'tcx> FnCtxt<'a, 'gcx, 'tcx> {
    pub fn new(inh: &'a Inherited<'a, 'gcx, 'tcx>,
               rty: Option<Ty<'tcx>>,
               body_id: ast::NodeId)
               -> FnCtxt<'a, 'gcx, 'tcx> {
        FnCtxt {
            ast_ty_to_ty_cache: RefCell::new(NodeMap()),
            body_id: body_id,
            err_count_on_creation: inh.tcx.sess.err_count(),
            ret_ty: rty,
            ps: RefCell::new(UnsafetyState::function(hir::Unsafety::Normal,
                                                     ast::CRATE_NODE_ID)),
            diverges: Cell::new(Diverges::Maybe),
            has_errors: Cell::new(false),
            enclosing_loops: RefCell::new(EnclosingLoops {
                stack: Vec::new(),
                by_id: NodeMap(),
            }),
            inh: inh,
        }
    }

    pub fn sess(&self) -> &Session {
        &self.tcx.sess
    }

    pub fn err_count_since_creation(&self) -> usize {
        self.tcx.sess.err_count() - self.err_count_on_creation
    }

    /// Produce warning on the given node, if the current point in the
    /// function is unreachable, and there hasn't been another warning.
    fn warn_if_unreachable(&self, id: ast::NodeId, span: Span, kind: &str) {
        if self.diverges.get() == Diverges::Always {
            self.diverges.set(Diverges::WarnedAlways);

            self.tables.borrow_mut().lints.add_lint(
                lint::builtin::UNREACHABLE_CODE,
                id, span,
                format!("unreachable {}", kind));
        }
    }

    pub fn cause(&self,
                 span: Span,
                 code: ObligationCauseCode<'tcx>)
                 -> ObligationCause<'tcx> {
        ObligationCause::new(span, self.body_id, code)
    }

    pub fn misc(&self, span: Span) -> ObligationCause<'tcx> {
        self.cause(span, ObligationCauseCode::MiscObligation)
    }

    /// Resolves type variables in `ty` if possible. Unlike the infcx
    /// version (resolve_type_vars_if_possible), this version will
    /// also select obligations if it seems useful, in an effort
    /// to get more type information.
    fn resolve_type_vars_with_obligations(&self, mut ty: Ty<'tcx>) -> Ty<'tcx> {
        debug!("resolve_type_vars_with_obligations(ty={:?})", ty);

        // No TyInfer()? Nothing needs doing.
        if !ty.has_infer_types() {
            debug!("resolve_type_vars_with_obligations: ty={:?}", ty);
            return ty;
        }

        // If `ty` is a type variable, see whether we already know what it is.
        ty = self.resolve_type_vars_if_possible(&ty);
        if !ty.has_infer_types() {
            debug!("resolve_type_vars_with_obligations: ty={:?}", ty);
            return ty;
        }

        // If not, try resolving pending obligations as much as
        // possible. This can help substantially when there are
        // indirect dependencies that don't seem worth tracking
        // precisely.
        self.select_obligations_where_possible();
        ty = self.resolve_type_vars_if_possible(&ty);

        debug!("resolve_type_vars_with_obligations: ty={:?}", ty);
        ty
    }

    fn record_deferred_call_resolution(&self,
                                       closure_def_id: DefId,
                                       r: DeferredCallResolutionHandler<'gcx, 'tcx>) {
        let mut deferred_call_resolutions = self.deferred_call_resolutions.borrow_mut();
        deferred_call_resolutions.entry(closure_def_id).or_insert(vec![]).push(r);
    }

    fn remove_deferred_call_resolutions(&self,
                                        closure_def_id: DefId)
                                        -> Vec<DeferredCallResolutionHandler<'gcx, 'tcx>>
    {
        let mut deferred_call_resolutions = self.deferred_call_resolutions.borrow_mut();
        deferred_call_resolutions.remove(&closure_def_id).unwrap_or(Vec::new())
    }

    pub fn tag(&self) -> String {
        let self_ptr: *const FnCtxt = self;
        format!("{:?}", self_ptr)
    }

    pub fn local_ty(&self, span: Span, nid: ast::NodeId) -> Ty<'tcx> {
        match self.locals.borrow().get(&nid) {
            Some(&t) => t,
            None => {
                span_bug!(span, "no type for local variable {}",
                          self.tcx.hir.node_to_string(nid));
            }
        }
    }

    #[inline]
    pub fn write_ty(&self, node_id: ast::NodeId, ty: Ty<'tcx>) {
        debug!("write_ty({}, {:?}) in fcx {}",
               node_id, ty, self.tag());
        self.tables.borrow_mut().node_types.insert(node_id, ty);

        if ty.references_error() {
            self.has_errors.set(true);
            self.set_tainted_by_errors();
        }

        // FIXME(canndrew): This is_never should probably be an is_uninhabited
        if ty.is_never() || self.type_var_diverges(ty) {
            self.diverges.set(self.diverges.get() | Diverges::Always);
        }
    }

    pub fn write_substs(&self, node_id: ast::NodeId, substs: ty::ItemSubsts<'tcx>) {
        if !substs.substs.is_noop() {
            debug!("write_substs({}, {:?}) in fcx {}",
                   node_id,
                   substs,
                   self.tag());

            self.tables.borrow_mut().item_substs.insert(node_id, substs);
        }
    }

    pub fn write_autoderef_adjustment(&self,
                                      node_id: ast::NodeId,
                                      derefs: usize,
                                      adjusted_ty: Ty<'tcx>) {
        self.write_adjustment(node_id, adjustment::Adjustment {
            kind: adjustment::Adjust::DerefRef {
                autoderefs: derefs,
                autoref: None,
                unsize: false
            },
            target: adjusted_ty
        });
    }

    pub fn write_adjustment(&self,
                            node_id: ast::NodeId,
                            adj: adjustment::Adjustment<'tcx>) {
        debug!("write_adjustment(node_id={}, adj={:?})", node_id, adj);

        if adj.is_identity() {
            return;
        }

        self.tables.borrow_mut().adjustments.insert(node_id, adj);
    }

    /// Basically whenever we are converting from a type scheme into
    /// the fn body space, we always want to normalize associated
    /// types as well. This function combines the two.
    fn instantiate_type_scheme<T>(&self,
                                  span: Span,
                                  substs: &Substs<'tcx>,
                                  value: &T)
                                  -> T
        where T : TypeFoldable<'tcx>
    {
        let value = value.subst(self.tcx, substs);
        let result = self.normalize_associated_types_in(span, &value);
        debug!("instantiate_type_scheme(value={:?}, substs={:?}) = {:?}",
               value,
               substs,
               result);
        result
    }

    /// As `instantiate_type_scheme`, but for the bounds found in a
    /// generic type scheme.
    fn instantiate_bounds(&self, span: Span, def_id: DefId, substs: &Substs<'tcx>)
                          -> ty::InstantiatedPredicates<'tcx> {
        let bounds = self.tcx.item_predicates(def_id);
        let result = bounds.instantiate(self.tcx, substs);
        let result = self.normalize_associated_types_in(span, &result.predicates);
        debug!("instantiate_bounds(bounds={:?}, substs={:?}) = {:?}",
               bounds,
               substs,
               result);
        ty::InstantiatedPredicates {
            predicates: result
        }
    }

    /// Replace all anonymized types with fresh inference variables
    /// and record them for writeback.
    fn instantiate_anon_types<T: TypeFoldable<'tcx>>(&self, value: &T) -> T {
        value.fold_with(&mut BottomUpFolder { tcx: self.tcx, fldop: |ty| {
            if let ty::TyAnon(def_id, substs) = ty.sty {
                // Use the same type variable if the exact same TyAnon appears more
                // than once in the return type (e.g. if it's pased to a type alias).
                let id = self.tcx.hir.as_local_node_id(def_id).unwrap();
                if let Some(ty_var) = self.anon_types.borrow().get(&id) {
                    return ty_var;
                }
                let span = self.tcx.def_span(def_id);
                let ty_var = self.next_ty_var(TypeVariableOrigin::TypeInference(span));
                self.anon_types.borrow_mut().insert(id, ty_var);

                let item_predicates = self.tcx.item_predicates(def_id);
                let bounds = item_predicates.instantiate(self.tcx, substs);

                for predicate in bounds.predicates {
                    // Change the predicate to refer to the type variable,
                    // which will be the concrete type, instead of the TyAnon.
                    // This also instantiates nested `impl Trait`.
                    let predicate = self.instantiate_anon_types(&predicate);

                    // Require that the predicate holds for the concrete type.
                    let cause = traits::ObligationCause::new(span, self.body_id,
                                                             traits::ReturnType);
                    self.register_predicate(traits::Obligation::new(cause, predicate));
                }

                ty_var
            } else {
                ty
            }
        }})
    }

    fn normalize_associated_types_in<T>(&self, span: Span, value: &T) -> T
        where T : TypeFoldable<'tcx>
    {
        self.inh.normalize_associated_types_in(span, self.body_id, value)
    }

    pub fn write_nil(&self, node_id: ast::NodeId) {
        self.write_ty(node_id, self.tcx.mk_nil());
    }

    pub fn write_error(&self, node_id: ast::NodeId) {
        self.write_ty(node_id, self.tcx.types.err);
    }

    pub fn require_type_meets(&self,
                              ty: Ty<'tcx>,
                              span: Span,
                              code: traits::ObligationCauseCode<'tcx>,
                              def_id: DefId)
    {
        self.register_bound(
            ty,
            def_id,
            traits::ObligationCause::new(span, self.body_id, code));
    }

    pub fn require_type_is_sized(&self,
                                 ty: Ty<'tcx>,
                                 span: Span,
                                 code: traits::ObligationCauseCode<'tcx>)
    {
        let lang_item = self.tcx.require_lang_item(lang_items::SizedTraitLangItem);
        self.require_type_meets(ty, span, code, lang_item);
    }

    pub fn register_bound(&self,
                          ty: Ty<'tcx>,
                          def_id: DefId,
                          cause: traits::ObligationCause<'tcx>)
    {
        self.fulfillment_cx.borrow_mut()
            .register_bound(self, ty, def_id, cause);
    }

    pub fn register_predicate(&self,
                              obligation: traits::PredicateObligation<'tcx>)
    {
        debug!("register_predicate({:?})", obligation);
        if obligation.has_escaping_regions() {
            span_bug!(obligation.cause.span, "escaping regions in predicate {:?}",
                      obligation);
        }
        self.fulfillment_cx
            .borrow_mut()
            .register_predicate_obligation(self, obligation);
    }

    pub fn register_predicates(&self,
                               obligations: Vec<traits::PredicateObligation<'tcx>>)
    {
        for obligation in obligations {
            self.register_predicate(obligation);
        }
    }

    pub fn register_infer_ok_obligations<T>(&self, infer_ok: InferOk<'tcx, T>) -> T {
        self.register_predicates(infer_ok.obligations);
        infer_ok.value
    }

    pub fn to_ty(&self, ast_t: &hir::Ty) -> Ty<'tcx> {
        let t = AstConv::ast_ty_to_ty(self, ast_t);
        self.register_wf_obligation(t, ast_t.span, traits::MiscObligation);
        t
    }

    pub fn node_ty(&self, id: ast::NodeId) -> Ty<'tcx> {
        match self.tables.borrow().node_types.get(&id) {
            Some(&t) => t,
            None if self.err_count_since_creation() != 0 => self.tcx.types.err,
            None => {
                bug!("no type for node {}: {} in fcx {}",
                     id, self.tcx.hir.node_to_string(id),
                     self.tag());
            }
        }
    }

    pub fn opt_node_ty_substs<F>(&self,
                                 id: ast::NodeId,
                                 f: F) where
        F: FnOnce(&ty::ItemSubsts<'tcx>),
    {
        if let Some(s) = self.tables.borrow().item_substs.get(&id) {
            f(s);
        }
    }

    /// Registers an obligation for checking later, during regionck, that the type `ty` must
    /// outlive the region `r`.
    pub fn register_region_obligation(&self,
                                      ty: Ty<'tcx>,
                                      region: &'tcx ty::Region,
                                      cause: traits::ObligationCause<'tcx>)
    {
        let mut fulfillment_cx = self.fulfillment_cx.borrow_mut();
        fulfillment_cx.register_region_obligation(ty, region, cause);
    }

    /// Registers an obligation for checking later, during regionck, that the type `ty` must
    /// outlive the region `r`.
    pub fn register_wf_obligation(&self,
                                  ty: Ty<'tcx>,
                                  span: Span,
                                  code: traits::ObligationCauseCode<'tcx>)
    {
        // WF obligations never themselves fail, so no real need to give a detailed cause:
        let cause = traits::ObligationCause::new(span, self.body_id, code);
        self.register_predicate(traits::Obligation::new(cause, ty::Predicate::WellFormed(ty)));
    }

    pub fn register_old_wf_obligation(&self,
                                      ty: Ty<'tcx>,
                                      span: Span,
                                      code: traits::ObligationCauseCode<'tcx>)
    {
        // Registers an "old-style" WF obligation that uses the
        // implicator code.  This is basically a buggy version of
        // `register_wf_obligation` that is being kept around
        // temporarily just to help with phasing in the newer rules.
        //
        // FIXME(#27579) all uses of this should be migrated to register_wf_obligation eventually
        let cause = traits::ObligationCause::new(span, self.body_id, code);
        self.register_region_obligation(ty, self.tcx.mk_region(ty::ReEmpty), cause);
    }

    /// Registers obligations that all types appearing in `substs` are well-formed.
    pub fn add_wf_bounds(&self, substs: &Substs<'tcx>, expr: &hir::Expr)
    {
        for ty in substs.types() {
            self.register_wf_obligation(ty, expr.span, traits::MiscObligation);
        }
    }

    /// Given a fully substituted set of bounds (`generic_bounds`), and the values with which each
    /// type/region parameter was instantiated (`substs`), creates and registers suitable
    /// trait/region obligations.
    ///
    /// For example, if there is a function:
    ///
    /// ```
    /// fn foo<'a,T:'a>(...)
    /// ```
    ///
    /// and a reference:
    ///
    /// ```
    /// let f = foo;
    /// ```
    ///
    /// Then we will create a fresh region variable `'$0` and a fresh type variable `$1` for `'a`
    /// and `T`. This routine will add a region obligation `$1:'$0` and register it locally.
    pub fn add_obligations_for_parameters(&self,
                                          cause: traits::ObligationCause<'tcx>,
                                          predicates: &ty::InstantiatedPredicates<'tcx>)
    {
        assert!(!predicates.has_escaping_regions());

        debug!("add_obligations_for_parameters(predicates={:?})",
               predicates);

        for obligation in traits::predicates_for_generics(cause, predicates) {
            self.register_predicate(obligation);
        }
    }

    // FIXME(arielb1): use this instead of field.ty everywhere
    // Only for fields! Returns <none> for methods>
    // Indifferent to privacy flags
    pub fn field_ty(&self,
                    span: Span,
                    field: &'tcx ty::FieldDef,
                    substs: &Substs<'tcx>)
                    -> Ty<'tcx>
    {
        self.normalize_associated_types_in(span,
                                           &field.ty(self.tcx, substs))
    }

    fn check_casts(&self) {
        let mut deferred_cast_checks = self.deferred_cast_checks.borrow_mut();
        for cast in deferred_cast_checks.drain(..) {
            cast.check(self);
        }
    }

    /// Apply "fallbacks" to some types
    /// unconstrained types get replaced with ! or  () (depending on whether
    /// feature(never_type) is enabled), unconstrained ints with i32, and
    /// unconstrained floats with f64.
    fn default_type_parameters(&self) {
        use rustc::ty::error::UnconstrainedNumeric::Neither;
        use rustc::ty::error::UnconstrainedNumeric::{UnconstrainedInt, UnconstrainedFloat};

        // Defaulting inference variables becomes very dubious if we have
        // encountered type-checking errors. Therefore, if we think we saw
        // some errors in this function, just resolve all uninstanted type
        // varibles to TyError.
        if self.is_tainted_by_errors() {
            for ty in &self.unsolved_variables() {
                if let ty::TyInfer(_) = self.shallow_resolve(ty).sty {
                    debug!("default_type_parameters: defaulting `{:?}` to error", ty);
                    self.demand_eqtype(syntax_pos::DUMMY_SP, *ty, self.tcx().types.err);
                }
            }
            return;
        }

        for ty in &self.unsolved_variables() {
            let resolved = self.resolve_type_vars_if_possible(ty);
            if self.type_var_diverges(resolved) {
                debug!("default_type_parameters: defaulting `{:?}` to `!` because it diverges",
                       resolved);
                self.demand_eqtype(syntax_pos::DUMMY_SP, *ty,
                                   self.tcx.mk_diverging_default());
            } else {
                match self.type_is_unconstrained_numeric(resolved) {
                    UnconstrainedInt => {
                        debug!("default_type_parameters: defaulting `{:?}` to `i32`",
                               resolved);
                        self.demand_eqtype(syntax_pos::DUMMY_SP, *ty, self.tcx.types.i32)
                    },
                    UnconstrainedFloat => {
                        debug!("default_type_parameters: defaulting `{:?}` to `f32`",
                               resolved);
                        self.demand_eqtype(syntax_pos::DUMMY_SP, *ty, self.tcx.types.f64)
                    }
                    Neither => { }
                }
            }
        }
    }

    fn select_all_obligations_and_apply_defaults(&self) {
        if self.tcx.sess.features.borrow().default_type_parameter_fallback {
            self.new_select_all_obligations_and_apply_defaults();
        } else {
            self.old_select_all_obligations_and_apply_defaults();
        }
    }

    // Implements old type inference fallback algorithm
    fn old_select_all_obligations_and_apply_defaults(&self) {
        self.select_obligations_where_possible();
        self.default_type_parameters();
        self.select_obligations_where_possible();
    }

    fn new_select_all_obligations_and_apply_defaults(&self) {
        use rustc::ty::error::UnconstrainedNumeric::Neither;
        use rustc::ty::error::UnconstrainedNumeric::{UnconstrainedInt, UnconstrainedFloat};

        // For the time being this errs on the side of being memory wasteful but provides better
        // error reporting.
        // let type_variables = self.type_variables.clone();

        // There is a possibility that this algorithm will have to run an arbitrary number of times
        // to terminate so we bound it by the compiler's recursion limit.
        for _ in 0..self.tcx.sess.recursion_limit.get() {
            // First we try to solve all obligations, it is possible that the last iteration
            // has made it possible to make more progress.
            self.select_obligations_where_possible();

            let mut conflicts = Vec::new();

            // Collect all unsolved type, integral and floating point variables.
            let unsolved_variables = self.unsolved_variables();

            // We must collect the defaults *before* we do any unification. Because we have
            // directly attached defaults to the type variables any unification that occurs
            // will erase defaults causing conflicting defaults to be completely ignored.
            let default_map: FxHashMap<Ty<'tcx>, _> =
                unsolved_variables
                    .iter()
                    .filter_map(|t| self.default(t).map(|d| (*t, d)))
                    .collect();

            let mut unbound_tyvars = FxHashSet();

            debug!("select_all_obligations_and_apply_defaults: defaults={:?}", default_map);

            // We loop over the unsolved variables, resolving them and if they are
            // and unconstrainted numeric type we add them to the set of unbound
            // variables. We do this so we only apply literal fallback to type
            // variables without defaults.
            for ty in &unsolved_variables {
                let resolved = self.resolve_type_vars_if_possible(ty);
                if self.type_var_diverges(resolved) {
                    self.demand_eqtype(syntax_pos::DUMMY_SP, *ty,
                                       self.tcx.mk_diverging_default());
                } else {
                    match self.type_is_unconstrained_numeric(resolved) {
                        UnconstrainedInt | UnconstrainedFloat => {
                            unbound_tyvars.insert(resolved);
                        },
                        Neither => {}
                    }
                }
            }

            // We now remove any numeric types that also have defaults, and instead insert
            // the type variable with a defined fallback.
            for ty in &unsolved_variables {
                if let Some(_default) = default_map.get(ty) {
                    let resolved = self.resolve_type_vars_if_possible(ty);

                    debug!("select_all_obligations_and_apply_defaults: \
                            ty: {:?} with default: {:?}",
                             ty, _default);

                    match resolved.sty {
                        ty::TyInfer(ty::TyVar(_)) => {
                            unbound_tyvars.insert(ty);
                        }

                        ty::TyInfer(ty::IntVar(_)) | ty::TyInfer(ty::FloatVar(_)) => {
                            unbound_tyvars.insert(ty);
                            if unbound_tyvars.contains(resolved) {
                                unbound_tyvars.remove(resolved);
                            }
                        }

                        _ => {}
                    }
                }
            }

            // If there are no more fallbacks to apply at this point we have applied all possible
            // defaults and type inference will proceed as normal.
            if unbound_tyvars.is_empty() {
                break;
            }

            // Finally we go through each of the unbound type variables and unify them with
            // the proper fallback, reporting a conflicting default error if any of the
            // unifications fail. We know it must be a conflicting default because the
            // variable would only be in `unbound_tyvars` and have a concrete value if
            // it had been solved by previously applying a default.

            // We wrap this in a transaction for error reporting, if we detect a conflict
            // we will rollback the inference context to its prior state so we can probe
            // for conflicts and correctly report them.

            let _ = self.commit_if_ok(|_: &infer::CombinedSnapshot| {
                conflicts.extend(
                    self.apply_defaults_and_return_conflicts(&unbound_tyvars, &default_map, None)
                );

                // If there are conflicts we rollback, otherwise commit
                if conflicts.len() > 0 {
                    Err(())
                } else {
                    Ok(())
                }
            });

            // Loop through each conflicting default, figuring out the default that caused
            // a unification failure and then report an error for each.
            for (conflict, default) in conflicts {
                let conflicting_default =
                    self.apply_defaults_and_return_conflicts(
                            &unbound_tyvars,
                            &default_map,
                            Some(conflict)
                        )
                        .last()
                        .map(|(_, tv)| tv)
                        .unwrap_or(type_variable::Default {
                            ty: self.next_ty_var(
                                TypeVariableOrigin::MiscVariable(syntax_pos::DUMMY_SP)),
                            origin_span: syntax_pos::DUMMY_SP,
                            // what do I put here?
                            def_id: self.tcx.hir.local_def_id(ast::CRATE_NODE_ID)
                        });

                // This is to ensure that we elimnate any non-determinism from the error
                // reporting by fixing an order, it doesn't matter what order we choose
                // just that it is consistent.
                let (first_default, second_default) =
                    if default.def_id < conflicting_default.def_id {
                        (default, conflicting_default)
                    } else {
                        (conflicting_default, default)
                    };


                self.report_conflicting_default_types(
                    first_default.origin_span,
                    self.body_id,
                    first_default,
                    second_default)
            }
        }

        self.select_obligations_where_possible();
    }

    // For use in error handling related to default type parameter fallback. We explicitly
    // apply the default that caused conflict first to a local version of the type variable
    // table then apply defaults until we find a conflict. That default must be the one
    // that caused conflict earlier.
    fn apply_defaults_and_return_conflicts<'b>(
        &'b self,
        unbound_vars: &'b FxHashSet<Ty<'tcx>>,
        default_map: &'b FxHashMap<Ty<'tcx>, type_variable::Default<'tcx>>,
        conflict: Option<Ty<'tcx>>,
    ) -> impl Iterator<Item=(Ty<'tcx>, type_variable::Default<'tcx>)> + 'b {
        use rustc::ty::error::UnconstrainedNumeric::Neither;
        use rustc::ty::error::UnconstrainedNumeric::{UnconstrainedInt, UnconstrainedFloat};

        conflict.into_iter().chain(unbound_vars.iter().cloned()).flat_map(move |ty| {
            if self.type_var_diverges(ty) {
                self.demand_eqtype(syntax_pos::DUMMY_SP, ty,
                                   self.tcx.mk_diverging_default());
            } else {
                match self.type_is_unconstrained_numeric(ty) {
                    UnconstrainedInt => {
                        self.demand_eqtype(syntax_pos::DUMMY_SP, ty, self.tcx.types.i32)
                    },
                    UnconstrainedFloat => {
                        self.demand_eqtype(syntax_pos::DUMMY_SP, ty, self.tcx.types.f64)
                    },
                    Neither => {
                        if let Some(default) = default_map.get(ty) {
                            let default = default.clone();
                            let default_ty = self.normalize_associated_types_in(
                                default.origin_span, &default.ty);
                            match self.eq_types(false,
                                                &self.misc(default.origin_span),
                                                ty,
                                                default_ty) {
                                Ok(ok) => self.register_infer_ok_obligations(ok),
                                Err(_) => {
                                    return Some((ty, default));
                                }
                            }
                        }
                    }
                }
            }

            None
        })
    }

    fn select_all_obligations_or_error(&self) {
        debug!("select_all_obligations_or_error");

        // upvar inference should have ensured that all deferred call
        // resolutions are handled by now.
        assert!(self.deferred_call_resolutions.borrow().is_empty());

        self.select_all_obligations_and_apply_defaults();

        let mut fulfillment_cx = self.fulfillment_cx.borrow_mut();

        match fulfillment_cx.select_all_or_error(self) {
            Ok(()) => { }
            Err(errors) => { self.report_fulfillment_errors(&errors); }
        }
    }

    /// Select as many obligations as we can at present.
    fn select_obligations_where_possible(&self) {
        match self.fulfillment_cx.borrow_mut().select_where_possible(self) {
            Ok(()) => { }
            Err(errors) => { self.report_fulfillment_errors(&errors); }
        }
    }

    /// For the overloaded lvalue expressions (`*x`, `x[3]`), the trait
    /// returns a type of `&T`, but the actual type we assign to the
    /// *expression* is `T`. So this function just peels off the return
    /// type by one layer to yield `T`.
    fn make_overloaded_lvalue_return_type(&self,
                                          method: MethodCallee<'tcx>)
                                          -> ty::TypeAndMut<'tcx>
    {
        // extract method return type, which will be &T;
        // all LB regions should have been instantiated during method lookup
        let ret_ty = method.ty.fn_ret();
        let ret_ty = self.tcx.no_late_bound_regions(&ret_ty).unwrap();

        // method returns &T, but the type as visible to user is T, so deref
        ret_ty.builtin_deref(true, NoPreference).unwrap()
    }

    fn lookup_indexing(&self,
                       expr: &hir::Expr,
                       base_expr: &'gcx hir::Expr,
                       base_ty: Ty<'tcx>,
                       idx_ty: Ty<'tcx>,
                       lvalue_pref: LvaluePreference)
                       -> Option<(/*index type*/ Ty<'tcx>, /*element type*/ Ty<'tcx>)>
    {
        // FIXME(#18741) -- this is almost but not quite the same as the
        // autoderef that normal method probing does. They could likely be
        // consolidated.

        let mut autoderef = self.autoderef(base_expr.span, base_ty);

        while let Some((adj_ty, autoderefs)) = autoderef.next() {
            if let Some(final_mt) = self.try_index_step(
                MethodCall::expr(expr.id),
                expr, base_expr, adj_ty, autoderefs,
                false, lvalue_pref, idx_ty)
            {
                autoderef.finalize(lvalue_pref, Some(base_expr));
                return Some(final_mt);
            }

            if let ty::TyArray(element_ty, _) = adj_ty.sty {
                autoderef.finalize(lvalue_pref, Some(base_expr));
                let adjusted_ty = self.tcx.mk_slice(element_ty);
                return self.try_index_step(
                    MethodCall::expr(expr.id), expr, base_expr,
                    adjusted_ty, autoderefs, true, lvalue_pref, idx_ty);
            }
        }
        autoderef.unambiguous_final_ty();
        None
    }
