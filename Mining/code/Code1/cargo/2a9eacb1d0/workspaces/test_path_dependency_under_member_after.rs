fn test_path_dependency_under_member() {
    let p = project("foo")
        .file("ws/Cargo.toml", r#"
            [project]
            name = "ws"
            version = "0.1.0"
            authors = []

            [dependencies]
            foo = { path = "../foo" }

            [workspace]
        "#)
        .file("ws/src/lib.rs", r"extern crate foo; pub fn f() { foo::f() }")
        .file("foo/Cargo.toml", r#"
            [project]
            workspace = "../ws"
            name = "foo"
            version = "0.1.0"
            authors = []

            [dependencies]
            bar = { path = "./bar" }
        "#)
        .file("foo/src/lib.rs", "extern crate bar; pub fn f() { bar::f() }")
        .file("foo/bar/Cargo.toml", r#"
            [project]
            name = "bar"
            version = "0.1.0"
            authors = []
        "#)
        .file("foo/bar/src/lib.rs", "pub fn f() { }");
    p.build();

    assert_that(p.cargo("build").cwd(p.root().join("ws")),
                execs().with_status(0));

    assert_that(&p.root().join("foo/bar/Cargo.lock"), is_not(existing_file()));
    assert_that(&p.root().join("foo/bar/target"), is_not(existing_dir()));

    assert_that(p.cargo("build").cwd(p.root().join("foo/bar")),
                execs().with_status(0));
    // Ideally, `foo/bar` should be a member of the workspace,
    // because it is hierarchically under the workspace member.
    assert_that(&p.root().join("foo/bar/Cargo.lock"), existing_file());
    assert_that(&p.root().join("foo/bar/target"), existing_dir());
}
