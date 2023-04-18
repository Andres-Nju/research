fn cargo_compile_with_invalid_manifest() {
    let p = project("foo")
        .file("Cargo.toml", "");

    assert_that(p.cargo_process("build"),
        execs()
        .with_status(101)
        .with_stderr("\
[ERROR] failed to parse manifest at `[..]`

Caused by:
  no `package` or `project` section found.
"))
}
