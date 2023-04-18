    fn lookup_candidates<FilterFn>(&mut self,
                                   lookup_name: Name,
                                   namespace: Namespace,
                                   filter_fn: FilterFn) -> SuggestedCandidates
        where FilterFn: Fn(Def) -> bool {

        let mut lookup_results = Vec::new();
        let mut worklist = Vec::new();
        worklist.push((self.graph_root, Vec::new(), false));

        while let Some((in_module,
                        path_segments,
                        in_module_is_extern)) = worklist.pop() {
            self.populate_module_if_necessary(in_module);

            in_module.for_each_child(|ident, ns, name_binding| {

                // avoid imports entirely
                if name_binding.is_import() && !name_binding.is_extern_crate() { return; }

                // collect results based on the filter function
                if ident.name == lookup_name && ns == namespace {
                    if filter_fn(name_binding.def()) {
                        // create the path
                        let span = name_binding.span;
                        let mut segms = path_segments.clone();
                        segms.push(ident.into());
                        let path = Path {
                            span: span,
                            global: false,
                            segments: segms,
                        };
                        // the entity is accessible in the following cases:
                        // 1. if it's defined in the same crate, it's always
                        // accessible (since private entities can be made public)
                        // 2. if it's defined in another crate, it's accessible
                        // only if both the module is public and the entity is
                        // declared as public (due to pruning, we don't explore
                        // outside crate private modules => no need to check this)
                        if !in_module_is_extern || name_binding.vis == ty::Visibility::Public {
                            lookup_results.push(path);
                        }
                    }
                }

                // collect submodules to explore
                if let Some(module) = name_binding.module() {
                    // form the path
                    let mut path_segments = path_segments.clone();
                    path_segments.push(ident.into());

                    if !in_module_is_extern || name_binding.vis == ty::Visibility::Public {
                        // add the module to the lookup
                        let is_extern = in_module_is_extern || name_binding.is_extern_crate();
                        if !worklist.iter().any(|&(m, ..)| m.def() == module.def()) {
                            worklist.push((module, path_segments, is_extern));
                        }
                    }
                }
            })
        }

        SuggestedCandidates {
            name: lookup_name.as_str().to_string(),
            candidates: lookup_results,
        }
    }
