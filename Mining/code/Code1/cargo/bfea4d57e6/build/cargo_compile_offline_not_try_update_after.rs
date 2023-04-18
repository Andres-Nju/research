fn cargo_compile_offline_not_try_update() {
    let p = project()
        .at("bar")
        .file(
            "Cargo.toml",
            r#"
            [project]
            name = "bar"
            version = "0.1.0"

            [dependencies]
            not_cached_dep = "1.2.5"
        "#,
        )
        .file("src/lib.rs", "")
        .build();

    p.cargo("build -Zoffline")
        .masquerade_as_nightly_cargo()
        .with_status(101)
        .with_stderr(
            "\
error: no matching package named `not_cached_dep` found
location searched: registry `[..]`
required by package `bar v0.1.0 ([..])`
As a reminder, you're using offline mode (-Z offline) \
which can sometimes cause surprising resolution failures, \
if this error is too confusing you may wish to retry \
without the offline flag.",
        )
        .run();
}
