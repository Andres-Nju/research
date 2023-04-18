    pub(crate) fn apply_change(&mut self, change: AnalysisChange) {
        let _p = profile("RootDatabase::apply_change");
        log::info!("apply_change {:?}", change);
        {
            let _p = profile("RootDatabase::apply_change/cancellation");
            self.salsa_runtime_mut().synthetic_write(Durability::LOW);
        }
        if !change.new_roots.is_empty() {
            let mut local_roots = Vec::clone(&self.local_roots());
            for (root_id, is_local) in change.new_roots {
                let root = if is_local { SourceRoot::new() } else { SourceRoot::new_library() };
                let durability = durability(&root);
                self.set_source_root_with_durability(root_id, Arc::new(root), durability);
                if is_local {
                    local_roots.push(root_id);
                }
            }
            self.set_local_roots_with_durability(Arc::new(local_roots), Durability::HIGH);
        }

        for (root_id, root_change) in change.roots_changed {
            self.apply_root_change(root_id, root_change);
        }
        for (file_id, text) in change.files_changed {
            let source_root_id = self.file_source_root(file_id);
            let source_root = self.source_root(source_root_id);
            let durability = durability(&source_root);
            self.set_file_text_with_durability(file_id, text, durability)
        }
        if !change.libraries_added.is_empty() {
            let mut libraries = Vec::clone(&self.library_roots());
            for library in change.libraries_added {
                libraries.push(library.root_id);
                self.set_source_root_with_durability(
                    library.root_id,
                    Default::default(),
                    Durability::HIGH,
                );
                self.set_library_symbols_with_durability(
                    library.root_id,
                    Arc::new(library.symbol_index),
                    Durability::HIGH,
                );
                self.apply_root_change(library.root_id, library.root_change);
            }
            self.set_library_roots_with_durability(Arc::new(libraries), Durability::HIGH);
        }
        if let Some(crate_graph) = change.crate_graph {
            self.set_crate_graph_with_durability(Arc::new(crate_graph), Durability::HIGH)
        }

        Arc::make_mut(&mut self.debug_data).merge(change.debug_data)
    }
