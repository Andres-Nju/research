fn member_manifest_version_error() {
    let p = project()
        .file(
            "Cargo.toml",
            r#"
            [project]
            name = "foo"
            version = "0.1.0"
            authors = []

            [dependencies]
            bar = { path = "bar" }

            [workspace]
        "#,
        )
        .file("src/main.rs", "fn main() {}")
        .file(
            "bar/Cargo.toml",
            r#"
            [project]
            name = "bar"
            version = "0.1.0"
            authors = []

            [dependencies]
            i-dont-exist = "0.55"
        "#,
        )
        .file("bar/src/main.rs", "fn main() {}")
        .build();

    let config = Config::default().unwrap();
    let ws = Workspace::new(&p.root().join("Cargo.toml"), &config).unwrap();
    let compile_options = CompileOptions::new(&config, CompileMode::Build).unwrap();
    let member_bar = ws.members().find(|m| &*m.name() == "bar").unwrap();

    let error = ops::compile(&ws, &compile_options).map(|_| ()).unwrap_err();
    eprintln!("{:?}", error);

    let resolve_err: &ResolveError = error.downcast_ref().expect("Not a ResolveError");
    let package_path = resolve_err.package_path();
    assert_eq!(package_path.len(), 1, "package_path: {:?}", package_path);
    assert_eq!(package_path[0], member_bar.package_id());
}
