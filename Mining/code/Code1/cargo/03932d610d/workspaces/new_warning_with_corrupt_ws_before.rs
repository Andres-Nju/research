fn new_warning_with_corrupt_ws() {
    let p = project().file("Cargo.toml", "asdf").build();
    p.cargo("new bar")
        .with_stderr(
            "\
[WARNING] compiling this new crate may not work due to invalid workspace configuration

failed to parse manifest at `[..]foo/Cargo.toml`
Caused by:
  could not parse input as TOML
Caused by:
  expected an equals, found eof at line 1
     Created binary (application) `bar` package
",
        )
        .run();
}
