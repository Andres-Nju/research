pub fn make_tests(config: &Config) -> Vec<test::TestDescAndFn> {
    debug!("making tests from {:?}", config.src_base.display());
    let mut tests = Vec::new();
    collect_tests_from_dir(
        config,
        &config.src_base,
        &config.src_base,
        &PathBuf::new(),
        &mut tests,
    ).unwrap();
    tests
}
