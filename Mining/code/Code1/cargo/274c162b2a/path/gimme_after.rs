            pub fn gimme() -> &'static str {
                "zoidberg"
            }
        "#,
        ).build();

    p.cargo("build")
        .with_stderr(&format!(
            "[COMPILING] bar v0.5.0 ({}/bar)\n\
             [COMPILING] foo v0.5.0 ({})\n\
             [FINISHED] dev [unoptimized + debuginfo] target(s) in \
             [..]\n",
            p.url(),
            p.url()
        )).run();

    assert!(p.bin("foo").is_file());

    p.process(&p.bin("foo")).with_stdout("zoidberg\n").run();
}

#[test]
fn no_rebuild_dependency() {
    let p = project()
        .file(
            "Cargo.toml",
            r#"
            [project]

            name = "foo"
            version = "0.5.0"
            authors = ["wycats@example.com"]

            [dependencies.bar]
            path = "bar"
        "#,
        ).file("src/main.rs", "extern crate bar; fn main() { bar::bar() }")
        .file("bar/Cargo.toml", &basic_lib_manifest("bar"))
        .file("bar/src/bar.rs", "pub fn bar() {}")
        .build();
    // First time around we should compile both foo and bar
    p.cargo("build")
        .with_stderr(&format!(
            "[COMPILING] bar v0.5.0 ({}/bar)\n\
             [COMPILING] foo v0.5.0 ({})\n\
             [FINISHED] dev [unoptimized + debuginfo] target(s) \
             in [..]\n",
            p.url(),
            p.url()
        )).run();

    sleep_ms(1000);
    p.change_file(
        "src/main.rs",
        r#"
        extern crate bar;
        fn main() { bar::bar(); }
    "#,
    );
    // Don't compile bar, but do recompile foo.
    p.cargo("build")
        .with_stderr(
            "\
             [COMPILING] foo v0.5.0 ([..])\n\
             [FINISHED] dev [unoptimized + debuginfo] target(s) \
             in [..]\n",
        ).run();
}

#[test]
fn deep_dependencies_trigger_rebuild() {
    let p = project()
        .file(
            "Cargo.toml",
            r#"
            [project]

            name = "foo"
            version = "0.5.0"
            authors = ["wycats@example.com"]

            [dependencies.bar]
            path = "bar"
        "#,
        ).file("src/main.rs", "extern crate bar; fn main() { bar::bar() }")
        .file(
            "bar/Cargo.toml",
            r#"
            [project]

            name = "bar"
            version = "0.5.0"
            authors = ["wycats@example.com"]

            [lib]
            name = "bar"
            [dependencies.baz]
            path = "../baz"
        "#,
        ).file(
            "bar/src/bar.rs",
            "extern crate baz; pub fn bar() { baz::baz() }",
        ).file("baz/Cargo.toml", &basic_lib_manifest("baz"))
        .file("baz/src/baz.rs", "pub fn baz() {}")
        .build();
    p.cargo("build")
        .with_stderr(&format!(
            "[COMPILING] baz v0.5.0 ({}/baz)\n\
             [COMPILING] bar v0.5.0 ({}/bar)\n\
             [COMPILING] foo v0.5.0 ({})\n\
             [FINISHED] dev [unoptimized + debuginfo] target(s) \
             in [..]\n",
            p.url(),
            p.url(),
            p.url()
        )).run();
    p.cargo("build").with_stdout("").run();

    // Make sure an update to baz triggers a rebuild of bar
    //
    // We base recompilation off mtime, so sleep for at least a second to ensure
    // that this write will change the mtime.
    File::create(&p.root().join("baz/src/baz.rs"))
        .unwrap()
        .write_all(br#"pub fn baz() { println!("hello!"); }"#)
        .unwrap();
    sleep_ms(1000);
    p.cargo("build")
        .with_stderr(&format!(
            "[COMPILING] baz v0.5.0 ({}/baz)\n\
             [COMPILING] bar v0.5.0 ({}/bar)\n\
             [COMPILING] foo v0.5.0 ({})\n\
             [FINISHED] dev [unoptimized + debuginfo] target(s) \
             in [..]\n",
            p.url(),
            p.url(),
            p.url()
        )).run();

    // Make sure an update to bar doesn't trigger baz
    File::create(&p.root().join("bar/src/bar.rs"))
        .unwrap()
        .write_all(
            br#"
        extern crate baz;
        pub fn bar() { println!("hello!"); baz::baz(); }
    "#,
        ).unwrap();
    sleep_ms(1000);
    p.cargo("build")
        .with_stderr(&format!(
            "[COMPILING] bar v0.5.0 ({}/bar)\n\
             [COMPILING] foo v0.5.0 ({})\n\
             [FINISHED] dev [unoptimized + debuginfo] target(s) \
             in [..]\n",
            p.url(),
            p.url()
        )).run();
}
