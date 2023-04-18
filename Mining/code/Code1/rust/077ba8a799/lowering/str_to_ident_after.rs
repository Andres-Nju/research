    fn str_to_ident(&self, s: &'static str) -> Name {
        Symbol::gensym(s)
    }

    fn allow_internal_unstable(&self, reason: CompilerDesugaringKind, span: Span) -> Span
    {
        let mark = Mark::fresh(Mark::root());
        mark.set_expn_info(codemap::ExpnInfo {
            call_site: span,
            callee: codemap::NameAndSpan {
                format: codemap::CompilerDesugaring(reason),
                span: Some(span),
                allow_internal_unstable: true,
                allow_internal_unsafe: false,
            },
        });
        span.with_ctxt(SyntaxContext::empty().apply_mark(mark))
    }

    // Creates a new hir::GenericParam for every new lifetime and type parameter
    // encountered while evaluating `f`. Definitions are created with the parent
    // provided. If no `parent_id` is provided, no definitions will be returned.
    fn collect_in_band_defs<T, F>(
        &mut self,
        parent_id: Option<DefId>,
        f: F
    ) -> (Vec<hir::GenericParam>, T) where F: FnOnce(&mut LoweringContext) -> T
    {
        assert!(!self.is_collecting_in_band_lifetimes);
        assert!(self.lifetimes_to_define.is_empty());
        self.is_collecting_in_band_lifetimes = self.sess.features.borrow().in_band_lifetimes;

        assert!(self.in_band_ty_params.is_empty());

        let res = f(self);

        self.is_collecting_in_band_lifetimes = false;

        let in_band_ty_params = self.in_band_ty_params.split_off(0);
        let lifetimes_to_define = self.lifetimes_to_define.split_off(0);

        let mut params = match parent_id {
            Some(parent_id) => lifetimes_to_define.into_iter().map(|(span, name)| {
                    let def_node_id = self.next_id().node_id;

                    // Add a definition for the in-band lifetime def
                    self.resolver.definitions().create_def_with_parent(
                        parent_id.index,
                        def_node_id,
                        DefPathData::LifetimeDef(name.as_str()),
                        DefIndexAddressSpace::High,
                        Mark::root()
                    );

                    hir::GenericParam::Lifetime(hir::LifetimeDef {
                        lifetime: hir::Lifetime {
                            id: def_node_id,
                            span,
                            name: hir::LifetimeName::Name(name),
                        },
                        bounds: Vec::new().into(),
                        pure_wrt_drop: false,
                        in_band: true,
                    })
                }).collect(),
            None => Vec::new(),
        };

        params.extend(in_band_ty_params.into_iter().map(|tp| hir::GenericParam::Type(tp)));

        (params, res)
    }

    // Evaluates `f` with the lifetimes in `lt_defs` in-scope.
    // This is used to track which lifetimes have already been defined, and
    // which are new in-band lifetimes that need to have a definition created
    // for them.
    fn with_in_scope_lifetime_defs<T, F>(
        &mut self,
        lt_defs: &[LifetimeDef],
        f: F
    ) -> T where F: FnOnce(&mut LoweringContext) -> T
    {
        let old_len = self.in_scope_lifetimes.len();
        let lt_def_names = lt_defs.iter().map(|lt_def| lt_def.lifetime.ident.name);
        self.in_scope_lifetimes.extend(lt_def_names);

        let res = f(self);

        self.in_scope_lifetimes.truncate(old_len);
        res
    }

    // Same as the method above, but accepts `hir::LifetimeDef`s
    // instead of `ast::LifetimeDef`s.
    // This should only be used with generics that have already had their
    // in-band lifetimes added. In practice, this means that this function is
    // only used when lowering a child item of a trait or impl.
    fn with_parent_impl_lifetime_defs<T, F>(
        &mut self,
        lt_defs: &[hir::LifetimeDef],
        f: F
    ) -> T where F: FnOnce(&mut LoweringContext) -> T
    {
        let old_len = self.in_scope_lifetimes.len();
        let lt_def_names = lt_defs.iter().map(|lt_def| lt_def.lifetime.name.name());
        self.in_scope_lifetimes.extend(lt_def_names);

        let res = f(self);

        self.in_scope_lifetimes.truncate(old_len);
        res
    }

    // Appends in-band lifetime defs and argument-position `impl Trait` defs
    // to the existing set of generics.
    fn add_in_band_defs<F, T>(
        &mut self,
        generics: &Generics,
        parent_id: Option<DefId>,
        f: F
    ) -> (hir::Generics, T)
        where F: FnOnce(&mut LoweringContext) -> T
    {
        let (in_band_defs, (mut lowered_generics, res)) =
            self.with_in_scope_lifetime_defs(
                &generics.params
                    .iter()
                    .filter_map(|p| match *p {
                        GenericParam::Lifetime(ref ld) => Some(ld.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>(),
                |this| {
                    this.collect_in_band_defs(parent_id, |this| {
                        (this.lower_generics(generics), f(this))
                    })
                }
            );

        lowered_generics.params =
            lowered_generics.params.iter().cloned().chain(in_band_defs).collect();

        (lowered_generics, res)
    }

    fn with_catch_scope<T, F>(&mut self, catch_id: NodeId, f: F) -> T
        where F: FnOnce(&mut LoweringContext) -> T
    {
        let len = self.catch_scopes.len();
        self.catch_scopes.push(catch_id);

        let result = f(self);
        assert_eq!(len + 1, self.catch_scopes.len(),
            "catch scopes should be added and removed in stack order");

        self.catch_scopes.pop().unwrap();

        result
    }

    fn lower_body<F>(&mut self, decl: Option<&FnDecl>, f: F) -> hir::BodyId
        where F: FnOnce(&mut LoweringContext) -> hir::Expr
    {
        let prev = mem::replace(&mut self.is_generator, false);
        let result = f(self);
        let r = self.record_body(result, decl);
        self.is_generator = prev;
        return r
    }

    fn with_loop_scope<T, F>(&mut self, loop_id: NodeId, f: F) -> T
        where F: FnOnce(&mut LoweringContext) -> T
    {
        // We're no longer in the base loop's condition; we're in another loop.
        let was_in_loop_condition = self.is_in_loop_condition;
        self.is_in_loop_condition = false;

        let len = self.loop_scopes.len();
        self.loop_scopes.push(loop_id);

        let result = f(self);
        assert_eq!(len + 1, self.loop_scopes.len(),
            "Loop scopes should be added and removed in stack order");

        self.loop_scopes.pop().unwrap();

        self.is_in_loop_condition = was_in_loop_condition;

        result
    }

    fn with_loop_condition_scope<T, F>(&mut self, f: F) -> T
        where F: FnOnce(&mut LoweringContext) -> T
    {
        let was_in_loop_condition = self.is_in_loop_condition;
        self.is_in_loop_condition = true;

        let result = f(self);

        self.is_in_loop_condition = was_in_loop_condition;

        result
    }

    fn with_new_scopes<T, F>(&mut self, f: F) -> T
        where F: FnOnce(&mut LoweringContext) -> T
    {
        let was_in_loop_condition = self.is_in_loop_condition;
        self.is_in_loop_condition = false;

        let catch_scopes = mem::replace(&mut self.catch_scopes, Vec::new());
        let loop_scopes = mem::replace(&mut self.loop_scopes, Vec::new());
        let result = f(self);
        self.catch_scopes = catch_scopes;
        self.loop_scopes = loop_scopes;

        self.is_in_loop_condition = was_in_loop_condition;

        result
    }

    fn with_parent_def<T, F>(&mut self, parent_id: NodeId, f: F) -> T
        where F: FnOnce(&mut LoweringContext) -> T
    {
        let old_def = self.parent_def;
        self.parent_def = {
            let defs = self.resolver.definitions();
            Some(defs.opt_def_index(parent_id).unwrap())
        };

        let result = f(self);

        self.parent_def = old_def;
        result
    }

    fn def_key(&mut self, id: DefId) -> DefKey {
        if id.is_local() {
            self.resolver.definitions().def_key(id.index)
        } else {
            self.cstore.def_key(id)
        }
    }

    fn lower_ident(&mut self, ident: Ident) -> Name {
        let ident = ident.modern();
        if ident.ctxt == SyntaxContext::empty() {
            return ident.name;
        }
        *self.name_map.entry(ident).or_insert_with(|| Symbol::from_ident(ident))
    }

    fn lower_opt_sp_ident(&mut self, o_id: Option<Spanned<Ident>>) -> Option<Spanned<Name>> {
        o_id.map(|sp_ident| respan(sp_ident.span, sp_ident.node.name))
    }

    fn lower_loop_destination(&mut self, destination: Option<(NodeId, Spanned<Ident>)>)
        -> hir::Destination
    {
        match destination {
            Some((id, label_ident)) => {
                let target = if let Def::Label(loop_id) = self.expect_full_def(id) {
                    hir::LoopIdResult::Ok(self.lower_node_id(loop_id).node_id)
                } else {
                    hir::LoopIdResult::Err(hir::LoopIdError::UnresolvedLabel)
                };
                hir::Destination {
                    ident: Some(label_ident),
                    target_id: hir::ScopeTarget::Loop(target),
                }
            },
            None => {
                let loop_id = self.loop_scopes
                                  .last()
                                  .map(|innermost_loop_id| *innermost_loop_id);

                hir::Destination {
                    ident: None,
                    target_id: hir::ScopeTarget::Loop(
                        loop_id.map(|id| Ok(self.lower_node_id(id).node_id))
                               .unwrap_or(Err(hir::LoopIdError::OutsideLoopScope))
                               .into())
                }
            }
        }
    }

    fn lower_attrs(&mut self, attrs: &Vec<Attribute>) -> hir::HirVec<Attribute> {
        attrs.iter().map(|a| self.lower_attr(a)).collect::<Vec<_>>().into()
    }

    fn lower_attr(&mut self, attr: &Attribute) -> Attribute {
        Attribute {
            id: attr.id,
            style: attr.style,
            path: attr.path.clone(),
            tokens: self.lower_token_stream(attr.tokens.clone()),
            is_sugared_doc: attr.is_sugared_doc,
            span: attr.span,
        }
    }

    fn lower_token_stream(&mut self, tokens: TokenStream) -> TokenStream {
        tokens.into_trees()
            .flat_map(|tree| self.lower_token_tree(tree).into_trees())
            .collect()
    }

    fn lower_token_tree(&mut self, tree: TokenTree) -> TokenStream {
        match tree {
            TokenTree::Token(span, token) => {
                self.lower_token(token, span)
            }
            TokenTree::Delimited(span, delimited) => {
                TokenTree::Delimited(span, Delimited {
                    delim: delimited.delim,
                    tts: self.lower_token_stream(delimited.tts.into()).into(),
                }).into()
            }
        }
    }

    fn lower_token(&mut self, token: Token, span: Span) -> TokenStream {
        match token {
            Token::Interpolated(_) => {}
            other => return TokenTree::Token(span, other).into(),
        }

        let tts = token.interpolated_to_tokenstream(&self.sess.parse_sess, span);
        self.lower_token_stream(tts)
    }

    fn lower_arm(&mut self, arm: &Arm) -> hir::Arm {
        hir::Arm {
            attrs: self.lower_attrs(&arm.attrs),
            pats: arm.pats.iter().map(|x| self.lower_pat(x)).collect(),
            guard: arm.guard.as_ref().map(|ref x| P(self.lower_expr(x))),
            body: P(self.lower_expr(&arm.body)),
        }
    }

    fn lower_ty_binding(&mut self, b: &TypeBinding, itctx: ImplTraitContext) -> hir::TypeBinding {
        hir::TypeBinding {
            id: self.lower_node_id(b.id).node_id,
            name: self.lower_ident(b.ident),
            ty: self.lower_ty(&b.ty, itctx),
            span: b.span,
        }
    }

    fn lower_ty(&mut self, t: &Ty, itctx: ImplTraitContext) -> P<hir::Ty> {
        let kind = match t.node {
            TyKind::Infer => hir::TyInfer,
            TyKind::Err => hir::TyErr,
            TyKind::Slice(ref ty) => hir::TySlice(self.lower_ty(ty, itctx)),
            TyKind::Ptr(ref mt) => hir::TyPtr(self.lower_mt(mt, itctx)),
            TyKind::Rptr(ref region, ref mt) => {
                let span = t.span.with_hi(t.span.lo());
                let lifetime = match *region {
                    Some(ref lt) => self.lower_lifetime(lt),
                    None => self.elided_lifetime(span)
                };
                hir::TyRptr(lifetime, self.lower_mt(mt, itctx))
            }
            TyKind::BareFn(ref f) => {
                self.with_in_scope_lifetime_defs(
                    &f.generic_params
                        .iter()
                        .filter_map(|p| match *p {
                            GenericParam::Lifetime(ref ld) => Some(ld.clone()),
                            _ => None,
                        })
                        .collect::<Vec<_>>(),
                    |this| hir::TyBareFn(P(hir::BareFnTy {
                        generic_params: this.lower_generic_params(&f.generic_params, &NodeMap()),
                        unsafety: this.lower_unsafety(f.unsafety),
                        abi: f.abi,
                        decl: this.lower_fn_decl(&f.decl, None, false),
                        arg_names: this.lower_fn_args_to_names(&f.decl),
                    })))
            }
            TyKind::Never => hir::TyNever,
            TyKind::Tup(ref tys) => {
                hir::TyTup(tys.iter().map(|ty| self.lower_ty(ty, itctx)).collect())
            }
            TyKind::Paren(ref ty) => {
                return self.lower_ty(ty, itctx);
            }
            TyKind::Path(ref qself, ref path) => {
                let id = self.lower_node_id(t.id);
                let qpath = self.lower_qpath(t.id, qself, path, ParamMode::Explicit, itctx);
                return self.ty_path(id, t.span, qpath);
            }
            TyKind::ImplicitSelf => {
                hir::TyPath(hir::QPath::Resolved(None, P(hir::Path {
                    def: self.expect_full_def(t.id),
                    segments: hir_vec![
                        hir::PathSegment::from_name(keywords::SelfType.name())
                    ],
                    span: t.span,
                })))
            }
            TyKind::Array(ref ty, ref length) => {
                let length = self.lower_body(None, |this| this.lower_expr(length));
                hir::TyArray(self.lower_ty(ty, itctx), length)
            }
            TyKind::Typeof(ref expr) => {
                let expr = self.lower_body(None, |this| this.lower_expr(expr));
                hir::TyTypeof(expr)
            }
            TyKind::TraitObject(ref bounds, ..) => {
                let mut lifetime_bound = None;
                let bounds = bounds.iter().filter_map(|bound| {
                    match *bound {
                        TraitTyParamBound(ref ty, TraitBoundModifier::None) => {
                            Some(self.lower_poly_trait_ref(ty, itctx))
                        }
                        TraitTyParamBound(_, TraitBoundModifier::Maybe) => None,
                        RegionTyParamBound(ref lifetime) => {
                            if lifetime_bound.is_none() {
                                lifetime_bound = Some(self.lower_lifetime(lifetime));
                            }
                            None
                        }
                    }
                }).collect();
                let lifetime_bound = lifetime_bound.unwrap_or_else(|| {
                    self.elided_lifetime(t.span)
                });
                hir::TyTraitObject(bounds, lifetime_bound)
            }
            TyKind::ImplTrait(ref bounds) => {
                use syntax::feature_gate::{emit_feature_err, GateIssue};
                let span = t.span;
                match itctx {
                    ImplTraitContext::Existential => {
                        let has_feature = self.sess.features.borrow().conservative_impl_trait;
                        if !t.span.allows_unstable() && !has_feature {
                            emit_feature_err(&self.sess.parse_sess, "conservative_impl_trait",
                                             t.span, GateIssue::Language,
                                             "`impl Trait` in return position is experimental");
                        }
                        let def_index = self.resolver.definitions().opt_def_index(t.id).unwrap();
                        let hir_bounds = self.lower_bounds(bounds, itctx);
                        let (lifetimes, lifetime_defs) =
                            self.lifetimes_from_impl_trait_bounds(def_index, &hir_bounds);

                        hir::TyImplTraitExistential(hir::ExistTy {
                            generics: hir::Generics {
                                params: lifetime_defs,
                                where_clause: hir::WhereClause {
                                    id: self.next_id().node_id,
                                    predicates: Vec::new().into(),
                                },
                                span,
                            },
                            bounds: hir_bounds,
                        }, lifetimes)
                    },
                    ImplTraitContext::Universal(def_id) => {
                        let has_feature = self.sess.features.borrow().universal_impl_trait;
                        if !t.span.allows_unstable() && !has_feature {
                            emit_feature_err(&self.sess.parse_sess, "universal_impl_trait",
                                             t.span, GateIssue::Language,
                                             "`impl Trait` in argument position is experimental");
                        }

                        let def_node_id = self.next_id().node_id;

                        // Add a definition for the in-band TyParam
                        let def_index = self.resolver.definitions().create_def_with_parent(
                            def_id.index,
                            def_node_id,
                            DefPathData::ImplTrait,
                            DefIndexAddressSpace::High,
                            Mark::root()
                        );

                        let hir_bounds = self.lower_bounds(bounds, itctx);
                        // Set the name to `impl Bound1 + Bound2`
                        let name = Symbol::intern(&pprust::ty_to_string(t));
                        self.in_band_ty_params.push(hir::TyParam {
                            name,
                            id: def_node_id,
                            bounds: hir_bounds,
                            default: None,
                            span,
                            pure_wrt_drop: false,
                            synthetic: Some(hir::SyntheticTyParamKind::ImplTrait),
                        });

                        hir::TyPath(hir::QPath::Resolved(None, P(hir::Path {
                            span,
                            def: Def::TyParam(DefId::local(def_index)),
                            segments: hir_vec![hir::PathSegment::from_name(name)],
                        })))
                    },
                    ImplTraitContext::Disallowed => {
                        span_err!(self.sess, t.span, E0562,
                                  "`impl Trait` not allowed outside of function \
                                  and inherent method return types");
                        hir::TyErr
                    }
                }
            }
            TyKind::Mac(_) => panic!("TyMac should have been expanded by now."),
        };

        let LoweredNodeId { node_id, hir_id } = self.lower_node_id(t.id);
        P(hir::Ty {
            id: node_id,
            node: kind,
            span: t.span,
            hir_id,
        })
    }

    fn lifetimes_from_impl_trait_bounds(
        &mut self,
        parent_index: DefIndex,
        bounds: &hir::TyParamBounds
    ) -> (HirVec<hir::Lifetime>, HirVec<hir::GenericParam>) {

        // This visitor walks over impl trait bounds and creates defs for all lifetimes which
        // appear in the bounds, excluding lifetimes that are created within the bounds.
        // e.g. 'a, 'b, but not 'c in `impl for<'c> SomeTrait<'a, 'b, 'c>`
        struct ImplTraitLifetimeCollector<'r, 'a: 'r> {
            context: &'r mut LoweringContext<'a>,
            parent: DefIndex,
            collect_elided_lifetimes: bool,
            currently_bound_lifetimes: Vec<hir::LifetimeName>,
            already_defined_lifetimes: HashSet<hir::LifetimeName>,
            output_lifetimes: Vec<hir::Lifetime>,
            output_lifetime_params: Vec<hir::GenericParam>,
        }

        impl<'r, 'a: 'r, 'v> hir::intravisit::Visitor<'v> for ImplTraitLifetimeCollector<'r, 'a> {
            fn nested_visit_map<'this>(&'this mut self)
                -> hir::intravisit::NestedVisitorMap<'this, 'v> {
                hir::intravisit::NestedVisitorMap::None
