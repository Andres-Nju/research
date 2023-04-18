fn invalid_global_config() {
    let p = project("foo")
        .file(
            "Cargo.toml",
            r#"
            [package]
            name = "foo"
            version = "0.0.0"
            authors = []

            [dependencies]
            foo = "0.1.0"
        "#,
        )
        .file(".cargo/config", "4")
        .file("src/lib.rs", "")
        .build();

    assert_that(
        p.cargo("build").arg("-v"),
        execs().with_status(101).with_stderr(
            "\
[ERROR] Couldn't load Cargo configuration

Caused by:
  could not parse TOML configuration in `[..]`

Caused by:
  could not parse input as TOML

Caused by:
  expected an equals, found eof at line 1
",
        ),
    );
}
