fn changing_bin_features_caches_targets() {
    let p = project("foo")
        .file(
            "Cargo.toml",
            r#"
            [package]
            name = "foo"
            authors = []
            version = "0.0.1"

            [features]
            foo = []
        "#,
        )
        .file(
            "src/main.rs",
            r#"
            fn main() {
                let msg = if cfg!(feature = "foo") { "feature on" } else { "feature off" };
                println!("{}", msg);
            }
        "#,
        )
        .build();

    // Windows has a problem with replacing a binary that was just executed.
    // Unlinking it will succeed, but then attempting to immediately replace
    // it will sometimes fail with "Already Exists".
    // See https://github.com/rust-lang/cargo/issues/5481
    let foo_proc = |name: &str| {
        let src = p.bin("foo");
        let dst = p.bin(name);
        fs::hard_link(&src, &dst).expect("Failed to link foo");
        p.process(dst)
    };

    assert_that(
        p.cargo("build"),
        execs().with_status(0).with_stderr(
            "\
[COMPILING] foo v0.0.1 ([..])
[FINISHED] dev [unoptimized + debuginfo] target(s) in [..]
",
        ),
    );
    assert_that(
        foo_proc("off1"),
        execs().with_status(0).with_stdout("feature off"),
    );

    assert_that(
        p.cargo("build").arg("--features").arg("foo"),
        execs().with_status(0).with_stderr(
            "\
[COMPILING] foo v0.0.1 ([..])
[FINISHED] dev [unoptimized + debuginfo] target(s) in [..]
",
        ),
    );
    assert_that(
        foo_proc("on1"),
        execs().with_status(0).with_stdout("feature on"),
    );

    /* Targets should be cached from the first build */

    assert_that(
        p.cargo("build"),
        execs().with_status(0).with_stderr(
            "\
[FINISHED] dev [unoptimized + debuginfo] target(s) in [..]
",
        ),
    );
    assert_that(
        foo_proc("off2"),
        execs().with_status(0).with_stdout("feature off"),
    );

    assert_that(
        p.cargo("build").arg("--features").arg("foo"),
        execs().with_status(0).with_stderr(
            "\
[FINISHED] dev [unoptimized + debuginfo] target(s) in [..]
",
        ),
    );
    assert_that(
        foo_proc("on2"),
        execs().with_status(0).with_stdout("feature on"),
    );
}
