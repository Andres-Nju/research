fn doc_cap_lints() {
    if !is_nightly() {
        // This can be removed once intra_doc_link_resolution_failure fails on stable.
        return;
    }
    let a = git::new("a", |p| {
        p.file("Cargo.toml", &basic_lib_manifest("a"))
            .file("src/lib.rs", BAD_INTRA_LINK_LIB)
    });

    let p = project()
        .file(
            "Cargo.toml",
            &format!(
                r#"
            [package]
            name = "foo"
            version = "0.0.1"
            authors = []

            [dependencies]
            a = {{ git = '{}' }}
        "#,
                a.url()
            ),
        )
        .file("src/lib.rs", "")
        .build();

    p.cargo("doc")
        .with_stderr_unordered(
            "\
[UPDATING] git repository `[..]`
[DOCUMENTING] a v0.5.0 ([..])
[CHECKING] a v0.5.0 ([..])
[DOCUMENTING] foo v0.0.1 ([..])
[FINISHED] dev [..]
",
        )
        .run();

    p.root().join("target").rm_rf();

    p.cargo("doc -vv")
        .with_stderr_contains(
            "\
[WARNING] `[bad_link]` cannot be resolved[..]
",
        )
        .run();
}
