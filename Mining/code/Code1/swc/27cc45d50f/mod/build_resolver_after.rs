fn build_resolver(base_url: PathBuf, paths: CompiledPaths) -> Box<SwcImportResolver> {
    static CACHE: Lazy<DashMap<(PathBuf, CompiledPaths), SwcImportResolver, ahash::RandomState>> =
        Lazy::new(Default::default);

    if let Some(cached) = CACHE.get(&(base_url.clone(), paths.clone())) {
        return Box::new((*cached).clone());
    }

    let r = {
        let r = TsConfigResolver::new(
            NodeModulesResolver::new(Default::default(), Default::default(), true),
            base_url.clone(),
            paths.clone(),
        );
        let r = CachingResolver::new(40, r);

        let r = NodeImportResolver::new(r);
        Arc::new(r)
    };

    CACHE.insert((base_url, paths), r.clone());

    Box::new(r)
}
