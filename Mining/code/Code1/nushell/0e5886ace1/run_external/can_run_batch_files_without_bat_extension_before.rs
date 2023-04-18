fn can_run_batch_files_without_bat_extension() {
    Playground::setup(
        "run a Windows batch file without specifying the extension",
        |dirs, sandbox| {
            sandbox.with_files(vec![FileWithContent(
                "foo.bat",
                r#"
                @echo off
                echo Hello World
            "#,
            )]);

            let actual = nu!(cwd: dirs.test(), pipeline("foo"));
            assert!(actual.out.contains("Hello World"));
        },
    );
}
