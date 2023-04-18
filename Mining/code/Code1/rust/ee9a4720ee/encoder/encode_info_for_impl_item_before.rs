    fn encode_info_for_impl_item(&mut self, def_id: DefId) -> Entry<'tcx> {
        debug!("IsolatedEncoder::encode_info_for_impl_item({:?})", def_id);
        let tcx = self.tcx;

        let node_id = self.tcx.hir.as_local_node_id(def_id).unwrap();
        let ast_item = self.tcx.hir.expect_impl_item(node_id);
        let impl_item = self.tcx.associated_item(def_id);

        let container = match impl_item.defaultness {
            hir::Defaultness::Default { has_value: true } => AssociatedContainer::ImplDefault,
            hir::Defaultness::Final => AssociatedContainer::ImplFinal,
            hir::Defaultness::Default { has_value: false } =>
                span_bug!(ast_item.span, "impl items always have values (currently)"),
        };

        let kind = match impl_item.kind {
            ty::AssociatedKind::Const => {
                if let hir::ImplItemKind::Const(_, body_id) = ast_item.node {
                    let mir = self.tcx.at(ast_item.span).mir_const_qualif(def_id).0;

                    EntryKind::AssociatedConst(container,
                        self.const_qualif(mir, body_id),
                        self.encode_rendered_const_for_body(body_id))
                } else {
                    bug!()
                }
            }
            ty::AssociatedKind::Method => {
                let fn_data = if let hir::ImplItemKind::Method(ref sig, body) = ast_item.node {
                    FnData {
                        constness: sig.constness,
                        arg_names: self.encode_fn_arg_names_for_body(body),
                        sig: self.lazy(&tcx.fn_sig(def_id)),
                    }
                } else {
                    bug!()
                };
                EntryKind::Method(self.lazy(&MethodData {
                    fn_data,
                    container,
                    has_self: impl_item.method_has_self_argument,
                }))
            }
            ty::AssociatedKind::Type => EntryKind::AssociatedType(container)
        };

        let mir =
            if let hir::ImplItemKind::Const(..) = ast_item.node {
                true
            } else if let hir::ImplItemKind::Method(ref sig, _) = ast_item.node {
                let generics = self.tcx.generics_of(def_id);
                let types = generics.parent_types as usize + generics.types.len();
                let needs_inline = types > 0 || tcx.trans_fn_attrs(def_id).requests_inline() &&
                    !self.metadata_output_only();
                let is_const_fn = sig.constness == hir::Constness::Const;
                let always_encode_mir = self.tcx.sess.opts.debugging_opts.always_encode_mir;
                needs_inline || is_const_fn || always_encode_mir
            } else {
                false
            };

        Entry {
            kind,
            visibility: self.lazy(&impl_item.vis),
            span: self.lazy(&ast_item.span),
            attributes: self.encode_attributes(&ast_item.attrs),
            children: LazySeq::empty(),
            stability: self.encode_stability(def_id),
            deprecation: self.encode_deprecation(def_id),

            ty: Some(self.encode_item_type(def_id)),
            inherent_impls: LazySeq::empty(),
            variances: if impl_item.kind == ty::AssociatedKind::Method {
                self.encode_variances_of(def_id)
            } else {
                LazySeq::empty()
            },
            generics: Some(self.encode_generics(def_id)),
            predicates: Some(self.encode_predicates(def_id)),

            mir: if mir { self.encode_optimized_mir(def_id) } else { None },
        }
    }

    fn encode_fn_arg_names_for_body(&mut self, body_id: hir::BodyId)
                                    -> LazySeq<ast::Name> {
        self.tcx.dep_graph.with_ignore(|| {
            let body = self.tcx.hir.body(body_id);
            self.lazy_seq(body.arguments.iter().map(|arg| {
                match arg.pat.node {
                    PatKind::Binding(_, _, name, _) => name.node,
                    _ => Symbol::intern("")
                }
            }))
        })
    }

    fn encode_fn_arg_names(&mut self, names: &[Spanned<ast::Name>])
                           -> LazySeq<ast::Name> {
        self.lazy_seq(names.iter().map(|name| name.node))
    }

    fn encode_optimized_mir(&mut self, def_id: DefId) -> Option<Lazy<mir::Mir<'tcx>>> {
        debug!("EntryBuilder::encode_mir({:?})", def_id);
        if self.tcx.mir_keys(LOCAL_CRATE).contains(&def_id) {
            let mir = self.tcx.optimized_mir(def_id);
            Some(self.lazy(&mir))
        } else {
            None
        }
    }
