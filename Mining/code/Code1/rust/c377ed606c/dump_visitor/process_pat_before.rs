    fn process_pat(&mut self, p: &'l ast::Pat) {
        match p.kind {
            PatKind::Struct(ref _path, ref fields, _) => {
                // FIXME do something with _path?
                let hir_id = self.tcx.hir().node_to_hir_id(p.id);
                let adt = match self.save_ctxt.tables.node_type_opt(hir_id) {
                    Some(ty) => ty.ty_adt_def().unwrap(),
                    None => {
                        visit::walk_pat(self, p);
                        return;
                    }
                };
                let variant = adt.variant_of_res(self.save_ctxt.get_path_res(p.id));

                for field in fields {
                    if let Some(index) = self.tcx.find_field_index(field.ident, variant) {
                        if !self.span.filter_generated(field.ident.span) {
                            let span = self.span_from_span(field.ident.span);
                            self.dumper.dump_ref(Ref {
                                kind: RefKind::Variable,
                                span,
                                ref_id: id_from_def_id(variant.fields[index].did),
                            });
                        }
                    }
                    self.visit_pat(&field.pat);
                }
            }
            _ => visit::walk_pat(self, p),
        }
    }

    fn process_var_decl(&mut self, pat: &'l ast::Pat) {
        // The pattern could declare multiple new vars,
        // we must walk the pattern and collect them all.
        let mut collector = PathCollector::new();
        collector.visit_pat(&pat);
        self.visit_pat(&pat);

        // Process collected paths.
        for (id, ident, _) in collector.collected_idents {
            match self.save_ctxt.get_path_res(id) {
                Res::Local(hir_id) => {
                    let id = self.tcx.hir().hir_to_node_id(hir_id);
                    let typ = self
                        .save_ctxt
                        .tables
                        .node_type_opt(hir_id)
                        .map(|t| t.to_string())
                        .unwrap_or_default();

                    // Rust uses the id of the pattern for var lookups, so we'll use it too.
                    if !self.span.filter_generated(ident.span) {
                        let qualname = format!("{}${}", ident.to_string(), id);
                        let id = id_from_node_id(id, &self.save_ctxt);
                        let span = self.span_from_span(ident.span);

                        self.dumper.dump_def(
                            &Access { public: false, reachable: false },
                            Def {
                                kind: DefKind::Local,
                                id,
                                span,
                                name: ident.to_string(),
                                qualname,
                                value: typ,
                                parent: None,
                                children: vec![],
                                decl_id: None,
                                docs: String::new(),
                                sig: None,
                                attributes: vec![],
                            },
                        );
                    }
                }
                Res::Def(HirDefKind::Ctor(..), _)
                | Res::Def(HirDefKind::Const, _)
                | Res::Def(HirDefKind::AssocConst, _)
                | Res::Def(HirDefKind::Struct, _)
                | Res::Def(HirDefKind::Variant, _)
                | Res::Def(HirDefKind::TyAlias, _)
                | Res::Def(HirDefKind::AssocTy, _)
                | Res::SelfTy(..) => {
                    self.dump_path_ref(id, &ast::Path::from_ident(ident));
                }
                def => {
                    error!("unexpected definition kind when processing collected idents: {:?}", def)
                }
            }
        }

        for (id, ref path) in collector.collected_paths {
            self.process_path(id, path);
        }
    }
