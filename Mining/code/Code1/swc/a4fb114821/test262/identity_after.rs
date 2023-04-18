fn identity() {
    let args: Vec<_> = env::args().collect();
    let mut tests = Vec::new();
    if !cfg!(target_os = "windows") {
        error_tests(&mut tests, true).expect("failed to load testss");
    }
    error_tests(&mut tests, false).expect("failed to load testss");
    test_main(&args, tests, Some(Options::new()));
}
