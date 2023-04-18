    pub fn populate_module_if_necessary(&mut self, module: Module<'b>) {
        if module.populated.get() { return }
        for child in self.session.cstore.item_children(module.def_id().unwrap()) {
            self.build_reduced_graph_for_external_crate_def(module, child);
        }
        module.populated.set(true)
    }

    fn import_extern_crate_macros(&mut self,
                                  extern_crate: &Item,
                                  module: Module<'b>,
                                  loaded_macros: LoadedMacros,
                                  legacy_imports: LegacyMacroImports,
                                  allow_shadowing: bool) {
        let import_macro = |this: &mut Self, name, ext: Rc<_>, span| {
            if let SyntaxExtension::NormalTT(..) = *ext {
                this.macro_names.insert(name);
            }
            if this.builtin_macros.insert(name, ext).is_some() && !allow_shadowing {
                let msg = format!("`{}` is already in scope", name);
                let note =
                    "macro-expanded `#[macro_use]`s may not shadow existing macros (see RFC 1560)";
                this.session.struct_span_err(span, &msg).note(note).emit();
            }
        };

        match loaded_macros {
            LoadedMacros::MacroRules(macros) => {
                let mark = Mark::fresh();
                if !macros.is_empty() {
                    let invocation = self.arenas.alloc_invocation_data(InvocationData {
                        module: Cell::new(module),
                        def_index: CRATE_DEF_INDEX,
                        const_integer: false,
                        legacy_scope: Cell::new(LegacyScope::Empty),
                        expansion: Cell::new(LegacyScope::Empty),
                    });
                    self.invocations.insert(mark, invocation);
                }

                let mut macros: FnvHashMap<_, _> = macros.into_iter().map(|mut def| {
                    def.body = mark_tts(&def.body, mark);
                    let ext = macro_rules::compile(&self.session.parse_sess, &def);
                    (def.ident.name, (def, Rc::new(ext)))
                }).collect();

                if let Some(span) = legacy_imports.import_all {
                    for (&name, &(_, ref ext)) in macros.iter() {
                        import_macro(self, name, ext.clone(), span);
                    }
                } else {
                    for (name, span) in legacy_imports.imports {
                        if let Some(&(_, ref ext)) = macros.get(&name) {
                            import_macro(self, name, ext.clone(), span);
                        } else {
                            span_err!(self.session, span, E0469, "imported macro not found");
                        }
                    }
                }
                for (name, span) in legacy_imports.reexports {
                    if let Some((mut def, _)) = macros.remove(&name) {
                        def.id = self.next_node_id();
                        self.exported_macros.push(def);
                    } else {
                        span_err!(self.session, span, E0470, "reexported macro not found");
                    }
                }
            }

            LoadedMacros::ProcMacros(macros) => {
                if !self.session.features.borrow().proc_macro {
                    let sess = &self.session.parse_sess;
                    let issue = feature_gate::GateIssue::Language;
                    let msg =
                        "loading custom derive macro crates is experimentally supported";
                    emit_feature_err(sess, "proc_macro", extern_crate.span, issue, msg);
                }
                if !legacy_imports.imports.is_empty() {
                    let msg = "`proc-macro` crates cannot be selectively imported from, \
                               must use `#[macro_use]`";
                    self.session.span_err(extern_crate.span, msg);
                }
                if !legacy_imports.reexports.is_empty() {
                    let msg = "`proc-macro` crates cannot be reexported from";
                    self.session.span_err(extern_crate.span, msg);
                }
                if let Some(span) = legacy_imports.import_all {
                    for (name, ext) in macros {
                        import_macro(self, name, Rc::new(ext), span);
                    }
                }
            }
        }
    }
