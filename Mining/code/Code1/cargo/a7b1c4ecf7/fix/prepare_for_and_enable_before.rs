fn prepare_for_and_enable() {
    if !is_nightly() {
        return;
    }
    let p = project()
        .file(
            "Cargo.toml",
            r#"
                [package]
                name = 'foo'
                version = '0.1.0'
                edition = '2018'
            "#,
        ).file("src/lib.rs", "")
        .build();

    let stderr = "\
error: cannot prepare for the 2018 edition when it is enabled, so cargo cannot
automatically fix errors in `src/lib.rs`

To prepare for the 2018 edition you should first remove `edition = '2018'` from
your `Cargo.toml` and then rerun this command. Once all warnings have been fixed
then you can re-enable the `edition` key in `Cargo.toml`. For some more
information about transitioning to the 2018 edition see:

  https://[..]

";
    p.cargo("fix --edition --allow-no-vcs")
        .with_stderr_contains(stderr)
        .with_status(101)
        .run();
}
