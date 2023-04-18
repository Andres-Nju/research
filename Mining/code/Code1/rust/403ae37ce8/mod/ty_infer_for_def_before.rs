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
            let default_map: FxHashMap<_, _> =
                unsolved_variables
                    .iter()
                    .filter_map(|t| self.default(t).map(|d| (t, d)))
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
        default_map: &'b FxHashMap<&'b Ty<'tcx>, type_variable::Default<'tcx>>,
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
                        if let Some(default) = default_map.get(&ty) {
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

    /// To type-check `base_expr[index_expr]`, we progressively autoderef
    /// (and otherwise adjust) `base_expr`, looking for a type which either
    /// supports builtin indexing or overloaded indexing.
    /// This loop implements one step in that search; the autoderef loop
    /// is implemented by `lookup_indexing`.
    fn try_index_step(&self,
                      method_call: MethodCall,
                      expr: &hir::Expr,
                      base_expr: &'gcx hir::Expr,
                      adjusted_ty: Ty<'tcx>,
                      autoderefs: usize,
                      unsize: bool,
                      lvalue_pref: LvaluePreference,
                      index_ty: Ty<'tcx>)
                      -> Option<(/*index type*/ Ty<'tcx>, /*element type*/ Ty<'tcx>)>
    {
        let tcx = self.tcx;
        debug!("try_index_step(expr={:?}, base_expr.id={:?}, adjusted_ty={:?}, \
                               autoderefs={}, unsize={}, index_ty={:?})",
               expr,
               base_expr,
               adjusted_ty,
               autoderefs,
               unsize,
               index_ty);

        let input_ty = self.next_ty_var(TypeVariableOrigin::AutoDeref(base_expr.span));

        // First, try built-in indexing.
        match (adjusted_ty.builtin_index(), &index_ty.sty) {
            (Some(ty), &ty::TyUint(ast::UintTy::Us)) | (Some(ty), &ty::TyInfer(ty::IntVar(_))) => {
                debug!("try_index_step: success, using built-in indexing");
                // If we had `[T; N]`, we should've caught it before unsizing to `[T]`.
                assert!(!unsize);
                self.write_autoderef_adjustment(base_expr.id, autoderefs, adjusted_ty);
                return Some((tcx.types.usize, ty));
            }
            _ => {}
        }

        // Try `IndexMut` first, if preferred.
        let method = match (lvalue_pref, tcx.lang_items.index_mut_trait()) {
            (PreferMutLvalue, Some(trait_did)) => {
                self.lookup_method_in_trait_adjusted(expr.span,
                                                     Some(&base_expr),
                                                     Symbol::intern("index_mut"),
                                                     trait_did,
                                                     autoderefs,
                                                     unsize,
                                                     adjusted_ty,
                                                     Some(vec![input_ty]))
            }
            _ => None,
        };

        // Otherwise, fall back to `Index`.
        let method = match (method, tcx.lang_items.index_trait()) {
            (None, Some(trait_did)) => {
                self.lookup_method_in_trait_adjusted(expr.span,
                                                     Some(&base_expr),
                                                     Symbol::intern("index"),
                                                     trait_did,
                                                     autoderefs,
                                                     unsize,
                                                     adjusted_ty,
                                                     Some(vec![input_ty]))
            }
            (method, _) => method,
        };

        // If some lookup succeeds, write callee into table and extract index/element
        // type from the method signature.
        // If some lookup succeeded, install method in table
        method.map(|method| {
            debug!("try_index_step: success, using overloaded indexing");
            self.tables.borrow_mut().method_map.insert(method_call, method);
            (input_ty, self.make_overloaded_lvalue_return_type(method).ty)
        })
    }

    fn check_method_argument_types(&self,
                                   sp: Span,
                                   method_fn_ty: Ty<'tcx>,
                                   callee_expr: &'gcx hir::Expr,
                                   args_no_rcvr: &'gcx [hir::Expr],
                                   tuple_arguments: TupleArgumentsFlag,
                                   expected: Expectation<'tcx>)
                                   -> Ty<'tcx> {
        if method_fn_ty.references_error() {
            let err_inputs = self.err_args(args_no_rcvr.len());

            let err_inputs = match tuple_arguments {
                DontTupleArguments => err_inputs,
                TupleArguments => vec![self.tcx.intern_tup(&err_inputs[..], false)],
            };

            self.check_argument_types(sp, &err_inputs[..], &[], args_no_rcvr,
                                      false, tuple_arguments, None);
            self.tcx.types.err
        } else {
            match method_fn_ty.sty {
                ty::TyFnDef(def_id, .., ref fty) => {
                    // HACK(eddyb) ignore self in the definition (see above).
                    let expected_arg_tys = self.expected_types_for_fn_args(
                        sp,
                        expected,
                        fty.0.output(),
                        &fty.0.inputs()[1..]
                    );
                    self.check_argument_types(sp, &fty.0.inputs()[1..], &expected_arg_tys[..],
                                              args_no_rcvr, fty.0.variadic, tuple_arguments,
                                              self.tcx.hir.span_if_local(def_id));
                    fty.0.output()
                }
                _ => {
                    span_bug!(callee_expr.span, "method without bare fn type");
                }
            }
        }
    }

    /// Generic function that factors out common logic from function calls,
    /// method calls and overloaded operators.
    fn check_argument_types(&self,
                            sp: Span,
                            fn_inputs: &[Ty<'tcx>],
                            expected_arg_tys: &[Ty<'tcx>],
                            args: &'gcx [hir::Expr],
                            variadic: bool,
                            tuple_arguments: TupleArgumentsFlag,
                            def_span: Option<Span>) {
        let tcx = self.tcx;

        // Grab the argument types, supplying fresh type variables
        // if the wrong number of arguments were supplied
        let supplied_arg_count = if tuple_arguments == DontTupleArguments {
            args.len()
        } else {
            1
        };

        // All the input types from the fn signature must outlive the call
        // so as to validate implied bounds.
        for &fn_input_ty in fn_inputs {
            self.register_wf_obligation(fn_input_ty, sp, traits::MiscObligation);
        }

        let mut expected_arg_tys = expected_arg_tys;
        let expected_arg_count = fn_inputs.len();

        let sp_args = if args.len() > 0 {
            let (first, args) = args.split_at(1);
            let mut sp_tmp = first[0].span;
            for arg in args {
                let sp_opt = self.sess().codemap().merge_spans(sp_tmp, arg.span);
                if ! sp_opt.is_some() {
                    break;
                }
                sp_tmp = sp_opt.unwrap();
            };
            sp_tmp
        } else {
            sp
        };

        fn parameter_count_error<'tcx>(sess: &Session, sp: Span, expected_count: usize,
                                       arg_count: usize, error_code: &str, variadic: bool,
                                       def_span: Option<Span>) {
            let mut err = sess.struct_span_err_with_code(sp,
                &format!("this function takes {}{} parameter{} but {} parameter{} supplied",
                    if variadic {"at least "} else {""},
                    expected_count,
                    if expected_count == 1 {""} else {"s"},
                    arg_count,
                    if arg_count == 1 {" was"} else {"s were"}),
                error_code);

            err.span_label(sp, &format!("expected {}{} parameter{}",
                                        if variadic {"at least "} else {""},
                                        expected_count,
                                        if expected_count == 1 {""} else {"s"}));
            if let Some(def_s) = def_span {
                err.span_label(def_s, &format!("defined here"));
            }
            err.emit();
        }

        let formal_tys = if tuple_arguments == TupleArguments {
            let tuple_type = self.structurally_resolved_type(sp, fn_inputs[0]);
            match tuple_type.sty {
                ty::TyTuple(arg_types, _) if arg_types.len() != args.len() => {
                    parameter_count_error(tcx.sess, sp_args, arg_types.len(), args.len(),
                                          "E0057", false, def_span);
                    expected_arg_tys = &[];
                    self.err_args(args.len())
                }
                ty::TyTuple(arg_types, _) => {
                    expected_arg_tys = match expected_arg_tys.get(0) {
                        Some(&ty) => match ty.sty {
                            ty::TyTuple(ref tys, _) => &tys,
                            _ => &[]
                        },
                        None => &[]
                    };
                    arg_types.to_vec()
                }
                _ => {
                    span_err!(tcx.sess, sp, E0059,
                        "cannot use call notation; the first type parameter \
                         for the function trait is neither a tuple nor unit");
                    expected_arg_tys = &[];
                    self.err_args(args.len())
                }
            }
        } else if expected_arg_count == supplied_arg_count {
            fn_inputs.to_vec()
        } else if variadic {
            if supplied_arg_count >= expected_arg_count {
                fn_inputs.to_vec()
            } else {
                parameter_count_error(tcx.sess, sp_args, expected_arg_count,
                                      supplied_arg_count, "E0060", true, def_span);
                expected_arg_tys = &[];
                self.err_args(supplied_arg_count)
            }
        } else {
            parameter_count_error(tcx.sess, sp_args, expected_arg_count,
                                  supplied_arg_count, "E0061", false, def_span);
            expected_arg_tys = &[];
            self.err_args(supplied_arg_count)
        };

        debug!("check_argument_types: formal_tys={:?}",
               formal_tys.iter().map(|t| self.ty_to_string(*t)).collect::<Vec<String>>());

        // Check the arguments.
        // We do this in a pretty awful way: first we typecheck any arguments
        // that are not closures, then we typecheck the closures. This is so
        // that we have more information about the types of arguments when we
        // typecheck the functions. This isn't really the right way to do this.
        for &check_closures in &[false, true] {
            debug!("check_closures={}", check_closures);

            // More awful hacks: before we check argument types, try to do
            // an "opportunistic" vtable resolution of any trait bounds on
            // the call. This helps coercions.
            if check_closures {
                self.select_obligations_where_possible();
            }

            // For variadic functions, we don't have a declared type for all of
            // the arguments hence we only do our usual type checking with
            // the arguments who's types we do know.
            let t = if variadic {
                expected_arg_count
            } else if tuple_arguments == TupleArguments {
                args.len()
            } else {
                supplied_arg_count
            };
            for (i, arg) in args.iter().take(t).enumerate() {
                // Warn only for the first loop (the "no closures" one).
                // Closure arguments themselves can't be diverging, but
                // a previous argument can, e.g. `foo(panic!(), || {})`.
                if !check_closures {
                    self.warn_if_unreachable(arg.id, arg.span, "expression");
                }

                let is_closure = match arg.node {
                    hir::ExprClosure(..) => true,
                    _ => false
                };

                if is_closure != check_closures {
                    continue;
                }

                debug!("checking the argument");
                let formal_ty = formal_tys[i];

                // The special-cased logic below has three functions:
                // 1. Provide as good of an expected type as possible.
                let expected = expected_arg_tys.get(i).map(|&ty| {
                    Expectation::rvalue_hint(self, ty)
                });

                let checked_ty = self.check_expr_with_expectation(&arg,
                                        expected.unwrap_or(ExpectHasType(formal_ty)));
                // 2. Coerce to the most detailed type that could be coerced
                //    to, which is `expected_ty` if `rvalue_hint` returns an
                //    `ExpectHasType(expected_ty)`, or the `formal_ty` otherwise.
                let coerce_ty = expected.and_then(|e| e.only_has_type(self));
                self.demand_coerce(&arg, checked_ty, coerce_ty.unwrap_or(formal_ty));

                // 3. Relate the expected type and the formal one,
                //    if the expected type was used for the coercion.
                coerce_ty.map(|ty| self.demand_suptype(arg.span, formal_ty, ty));
            }
        }

        // We also need to make sure we at least write the ty of the other
        // arguments which we skipped above.
        if variadic {
            for arg in args.iter().skip(expected_arg_count) {
                let arg_ty = self.check_expr(&arg);

                // There are a few types which get autopromoted when passed via varargs
                // in C but we just error out instead and require explicit casts.
                let arg_ty = self.structurally_resolved_type(arg.span,
                                                             arg_ty);
                match arg_ty.sty {
                    ty::TyFloat(ast::FloatTy::F32) => {
                        self.type_error_message(arg.span, |t| {
                            format!("can't pass an `{}` to variadic \
                                     function, cast to `c_double`", t)
                        }, arg_ty);
                    }
                    ty::TyInt(ast::IntTy::I8) | ty::TyInt(ast::IntTy::I16) | ty::TyBool => {
                        self.type_error_message(arg.span, |t| {
                            format!("can't pass `{}` to variadic \
                                     function, cast to `c_int`",
                                           t)
                        }, arg_ty);
                    }
                    ty::TyUint(ast::UintTy::U8) | ty::TyUint(ast::UintTy::U16) => {
                        self.type_error_message(arg.span, |t| {
                            format!("can't pass `{}` to variadic \
                                     function, cast to `c_uint`",
                                           t)
                        }, arg_ty);
                    }
                    ty::TyFnDef(.., f) => {
                        let ptr_ty = self.tcx.mk_fn_ptr(f);
                        let ptr_ty = self.resolve_type_vars_if_possible(&ptr_ty);
                        self.type_error_message(arg.span,
                                                |t| {
                            format!("can't pass `{}` to variadic \
                                     function, cast to `{}`", t, ptr_ty)
                        }, arg_ty);
                    }
                    _ => {}
                }
            }
        }
    }

    fn err_args(&self, len: usize) -> Vec<Ty<'tcx>> {
        (0..len).map(|_| self.tcx.types.err).collect()
    }

    // AST fragment checking
    fn check_lit(&self,
                 lit: &ast::Lit,
                 expected: Expectation<'tcx>)
                 -> Ty<'tcx>
    {
        let tcx = self.tcx;

        match lit.node {
            ast::LitKind::Str(..) => tcx.mk_static_str(),
            ast::LitKind::ByteStr(ref v) => {
                tcx.mk_imm_ref(tcx.mk_region(ty::ReStatic),
                                tcx.mk_array(tcx.types.u8, v.len()))
            }
            ast::LitKind::Byte(_) => tcx.types.u8,
            ast::LitKind::Char(_) => tcx.types.char,
            ast::LitKind::Int(_, ast::LitIntType::Signed(t)) => tcx.mk_mach_int(t),
            ast::LitKind::Int(_, ast::LitIntType::Unsigned(t)) => tcx.mk_mach_uint(t),
            ast::LitKind::Int(_, ast::LitIntType::Unsuffixed) => {
                let opt_ty = expected.to_option(self).and_then(|ty| {
                    match ty.sty {
                        ty::TyInt(_) | ty::TyUint(_) => Some(ty),
                        ty::TyChar => Some(tcx.types.u8),
                        ty::TyRawPtr(..) => Some(tcx.types.usize),
                        ty::TyFnDef(..) | ty::TyFnPtr(_) => Some(tcx.types.usize),
                        _ => None
                    }
                });
                opt_ty.unwrap_or_else(
                    || tcx.mk_int_var(self.next_int_var_id()))
            }
            ast::LitKind::Float(_, t) => tcx.mk_mach_float(t),
            ast::LitKind::FloatUnsuffixed(_) => {
                let opt_ty = expected.to_option(self).and_then(|ty| {
                    match ty.sty {
                        ty::TyFloat(_) => Some(ty),
                        _ => None
                    }
                });
                opt_ty.unwrap_or_else(
                    || tcx.mk_float_var(self.next_float_var_id()))
            }
            ast::LitKind::Bool(_) => tcx.types.bool
        }
    }

    fn check_expr_eq_type(&self,
                          expr: &'gcx hir::Expr,
                          expected: Ty<'tcx>) {
        let ty = self.check_expr_with_hint(expr, expected);
        self.demand_eqtype(expr.span, expected, ty);
    }

    pub fn check_expr_has_type(&self,
                               expr: &'gcx hir::Expr,
                               expected: Ty<'tcx>) -> Ty<'tcx> {
        let ty = self.check_expr_with_hint(expr, expected);
        self.demand_suptype(expr.span, expected, ty);
        ty
    }

    fn check_expr_coercable_to_type(&self,
                                    expr: &'gcx hir::Expr,
                                    expected: Ty<'tcx>) -> Ty<'tcx> {
        let ty = self.check_expr_with_hint(expr, expected);
        self.demand_coerce(expr, ty, expected);
        ty
    }

    fn check_expr_with_hint(&self, expr: &'gcx hir::Expr,
                            expected: Ty<'tcx>) -> Ty<'tcx> {
        self.check_expr_with_expectation(expr, ExpectHasType(expected))
    }

    fn check_expr_with_expectation(&self,
                                   expr: &'gcx hir::Expr,
                                   expected: Expectation<'tcx>) -> Ty<'tcx> {
        self.check_expr_with_expectation_and_lvalue_pref(expr, expected, NoPreference)
    }

    fn check_expr(&self, expr: &'gcx hir::Expr) -> Ty<'tcx> {
        self.check_expr_with_expectation(expr, NoExpectation)
    }

    fn check_expr_with_lvalue_pref(&self, expr: &'gcx hir::Expr,
                                   lvalue_pref: LvaluePreference) -> Ty<'tcx> {
        self.check_expr_with_expectation_and_lvalue_pref(expr, NoExpectation, lvalue_pref)
    }

    // determine the `self` type, using fresh variables for all variables
    // declared on the impl declaration e.g., `impl<A,B> for Vec<(A,B)>`
    // would return ($0, $1) where $0 and $1 are freshly instantiated type
    // variables.
    pub fn impl_self_ty(&self,
                        span: Span, // (potential) receiver for this impl
                        did: DefId)
                        -> TypeAndSubsts<'tcx> {
        let ity = self.tcx.item_type(did);
        debug!("impl_self_ty: ity={:?}", ity);

        let substs = self.fresh_substs_for_item(span, did);
        let substd_ty = self.instantiate_type_scheme(span, &substs, &ity);

        TypeAndSubsts { substs: substs, ty: substd_ty }
    }
