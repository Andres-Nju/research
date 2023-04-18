fn can_run_doc_tests() {
    Package::new("bar", "0.1.0").publish();
    Package::new("bar", "0.2.0").publish();

    let foo = project()
        .file(
            "Cargo.toml",
            r#"
            cargo-features = ["rename-dependency"]

            [project]
            name = "foo"
            version = "0.0.1"

            [dependencies]
            bar = { version = "0.1.0" }
            baz = { version = "0.2.0", package = "bar" }
        "#,
        ).file(
            "src/lib.rs",
            "
            extern crate bar;
            extern crate baz;
        ",
        ).build();

    foo.cargo("test -v")
        .masquerade_as_nightly_cargo()
        .with_stderr_contains(
            "\
[DOCTEST] foo
[RUNNING] `rustdoc --test [CWD]/src/lib.rs \
        [..] \
        --extern bar=[CWD]/target/debug/deps/libbar-[..].rlib \
        --extern baz=[CWD]/target/debug/deps/libbar-[..].rlib \
        [..]`
",
        ).run();
}
