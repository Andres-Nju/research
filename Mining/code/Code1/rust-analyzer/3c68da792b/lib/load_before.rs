pub fn load(
    source_roots: &FxHashMap<SourceRootId, PackageRoot>,
    crate_graph: CrateGraph,
    vfs: &mut Vfs,
    receiver: Receiver<VfsTask>,
) -> AnalysisHost {
    let lru_cap = std::env::var("RA_LRU_CAP").ok().and_then(|it| it.parse::<usize>().ok());
    let mut host = AnalysisHost::new(lru_cap, FeatureFlags::default());
    let mut analysis_change = AnalysisChange::new();
    analysis_change.set_crate_graph(crate_graph);

    // wait until Vfs has loaded all roots
    let mut roots_loaded = HashSet::new();
    for task in receiver {
        vfs.handle_task(task);
        let mut done = false;
        for change in vfs.commit_changes() {
            match change {
                VfsChange::AddRoot { root, files } => {
                    let source_root_id = vfs_root_to_id(root);
                    let is_local = source_roots[&source_root_id].is_member();
                    log::debug!(
                        "loaded source root {:?} with path {:?}",
                        source_root_id,
                        vfs.root2path(root)
                    );
                    analysis_change.add_root(source_root_id, is_local);

                    let mut file_map = FxHashMap::default();
                    for (vfs_file, path, text) in files {
                        let file_id = vfs_file_to_id(vfs_file);
                        analysis_change.add_file(source_root_id, file_id, path.clone(), text);
                        file_map.insert(path, file_id);
                    }
                    roots_loaded.insert(source_root_id);
                    if roots_loaded.len() == vfs.n_roots() {
                        done = true;
                    }
                }
                VfsChange::AddFile { .. }
                | VfsChange::RemoveFile { .. }
                | VfsChange::ChangeFile { .. } => {
                    // We just need the first scan, so just ignore these
                }
            }
        }
        if done {
            break;
        }
    }

    host.apply_change(analysis_change);
    host
}
