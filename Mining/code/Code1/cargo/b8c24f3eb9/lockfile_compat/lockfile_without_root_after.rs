fn lockfile_without_root() {
    Package::new("bar", "0.1.0").publish();

    let lockfile = r#"[[package]]
name = "bar"
version = "0.1.0"
source = "registry+https://github.com/rust-lang/crates.io-index"

[[package]]
name = "foo"
version = "0.0.1"
dependencies = [
 "bar 0.1.0 (registry+https://github.com/rust-lang/crates.io-index)",
]
"#;

    let p = project()
        .file(
            "Cargo.toml",
            r#"
            [package]
            name = "foo"
            version = "0.0.1"
            authors = []

            [dependencies]
            bar = "0.1.0"
        "#,
        ).file("src/lib.rs", "")
        .file("Cargo.lock", lockfile);

    let p = p.build();

    p.cargo("build").run();

    let lock = p.read_lockfile();
    assert!(lock.starts_with(lockfile.trim()));
}

#[test]
fn locked_correct_error() {
    Package::new("bar", "0.1.0").publish();

    let p = project()
        .file(
            "Cargo.toml",
            r#"
            [project]
            name = "foo"
            version = "0.0.1"
            authors = []

            [dependencies]
            bar = "0.1.0"
        "#,
        ).file("src/lib.rs", "");
    let p = p.build();

    p.cargo("build --locked")
        .with_status(101)
        .with_stderr(
            "\
[UPDATING] `[..]` index
error: the lock file [CWD]/Cargo.lock needs to be updated but --locked was passed to prevent this
",
        ).run();
}
