fn compile_offline_while_transitive_dep_not_cached() {
    let baz = Package::new("baz", "1.0.0");
    let baz_path = baz.archive_dst();
    baz.publish();

    let mut content = Vec::new();

    let mut file = File::open(baz_path.clone()).ok().unwrap();
    let _ok = file.read_to_end(&mut content).ok().unwrap();
    drop(file);
    drop(File::create(baz_path.clone()).ok().unwrap());

    Package::new("bar", "0.1.0").dep("baz", "1.0.0").publish();

    let p = project()
        .file(
            "Cargo.toml",
            r#"
            [project]
            name = "foo"
            version = "0.0.1"

            [dependencies]
            bar = "0.1.0"
        "#,
        )
        .file("src/main.rs", "fn main(){}")
        .build();

    // simulate download bar, but fail to download baz
    p.cargo("build")
        .with_status(101)
        .with_stderr_contains("[..]failed to verify the checksum of `baz[..]")
        .run();

    drop(File::create(baz_path).ok().unwrap().write_all(&content));

    p.cargo("build -Zoffline")
        .masquerade_as_nightly_cargo()
        .with_status(101)
        .with_stderr(
            "\
error: no matching package named `baz` found
location searched: registry `[..]`
required by package `bar v0.1.0`
    ... which is depended on by `foo v0.0.1 ([CWD])`
As a reminder, you're using offline mode (-Z offline) \
which can sometimes cause surprising resolution failures, \
if this error is too confusing you may with to retry \
without the offline flag.",
        )
        .run();
}
