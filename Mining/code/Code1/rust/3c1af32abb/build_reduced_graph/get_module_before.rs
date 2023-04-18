    pub fn get_module(&mut self, def_id: DefId) -> Module<'a> {
        if def_id.krate == LOCAL_CRATE {
            return self.module_map[&def_id]
        }

        let macros_only = self.session.cstore.dep_kind(def_id.krate).macros_only();
        if let Some(&module) = self.extern_module_map.get(&(def_id, macros_only)) {
            return module;
        }

        let (name, parent) = if def_id.index == CRATE_DEF_INDEX {
            (self.session.cstore.crate_name(def_id.krate), None)
        } else {
            let def_key = self.session.cstore.def_key(def_id);
            (def_key.disambiguated_data.data.get_opt_name().unwrap(),
             Some(self.get_module(DefId { index: def_key.parent.unwrap(), ..def_id })))
        };

        let kind = ModuleKind::Def(Def::Mod(def_id), name);
        self.arenas.alloc_module(ModuleData::new(parent, kind, def_id, Mark::root(), DUMMY_SP))
    }

    pub fn macro_def_scope(&mut self, expansion: Mark) -> Module<'a> {
        let def_id = self.macro_defs[&expansion];
        if let Some(id) = self.definitions.as_local_node_id(def_id) {
            self.local_macro_def_scopes[&id]
        } else if def_id.krate == BUILTIN_MACROS_CRATE {
            // FIXME(jseyfried): This happens when `include!()`ing a `$crate::` path, c.f, #40469.
            self.graph_root
        } else {
            let module_def_id = ty::DefIdTree::parent(&*self, def_id).unwrap();
            self.get_module(module_def_id)
        }
    }
