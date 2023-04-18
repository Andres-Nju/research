    pub fn apply_change(&mut self, change: AnalysisChange) {
        log::info!("apply_change {:?}", change);
        // self.gc_syntax_trees();

        for (file_id, text) in change.files_changed {
            self.db
                .query_mut(ra_db::FileTextQuery)
                .set(file_id, Arc::new(text))
        }
        if !(change.files_added.is_empty() && change.files_removed.is_empty()) {
            let file_resolver = change
                .file_resolver
                .expect("change resolver when changing set of files");
            let mut source_root = SourceRoot::clone(&self.db.source_root(WORKSPACE));
            for (file_id, text) in change.files_added {
                self.db
                    .query_mut(ra_db::FileTextQuery)
                    .set(file_id, Arc::new(text));
                self.db
                    .query_mut(ra_db::FileSourceRootQuery)
                    .set(file_id, ra_db::WORKSPACE);
                source_root.files.insert(file_id);
            }
            for file_id in change.files_removed {
                self.db
                    .query_mut(ra_db::FileTextQuery)
                    .set(file_id, Arc::new(String::new()));
                source_root.files.remove(&file_id);
            }
            source_root.file_resolver = file_resolver;
            self.db
                .query_mut(ra_db::SourceRootQuery)
                .set(WORKSPACE, Arc::new(source_root))
        }
        if !change.libraries_added.is_empty() {
            let mut libraries = Vec::clone(&self.db.libraries());
            for library in change.libraries_added {
                let source_root_id = SourceRootId(1 + libraries.len() as u32);
                libraries.push(source_root_id);
                let mut files = FxHashSet::default();
                for (file_id, text) in library.files {
                    files.insert(file_id);
                    log::debug!(
                        "library file: {:?} {:?}",
                        file_id,
                        library.file_resolver.debug_path(file_id)
                    );
                    self.db
                        .query_mut(ra_db::FileSourceRootQuery)
                        .set_constant(file_id, source_root_id);
                    self.db
                        .query_mut(ra_db::FileTextQuery)
                        .set_constant(file_id, Arc::new(text));
                }
                let source_root = SourceRoot {
                    files,
                    file_resolver: library.file_resolver,
                };
                self.db
                    .query_mut(ra_db::SourceRootQuery)
                    .set(source_root_id, Arc::new(source_root));
                self.db
                    .query_mut(crate::symbol_index::LibrarySymbolsQuery)
                    .set(source_root_id, Arc::new(library.symbol_index));
            }
            self.db
                .query_mut(ra_db::LibrariesQuery)
                .set((), Arc::new(libraries));
        }
        if let Some(crate_graph) = change.crate_graph {
            self.db
                .query_mut(ra_db::CrateGraphQuery)
                .set((), Arc::new(crate_graph))
        }
    }
