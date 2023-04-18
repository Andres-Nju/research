fn can_run_batch_files() {
    use nu_test_support::fs::Stub::FileWithContent;
    Playground::setup("run a Windows batch file", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "foo.cmd",
            r#"
                @echo off
                echo Hello World
            "#,
        )]);

        let actual = nu!(cwd: dirs.test(), pipeline("foo.cmd"));
        assert!(actual.out.contains("Hello World"));
    });
}
