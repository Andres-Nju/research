fn parse_failure_due_conflicted_flags() {
    Playground::setup("nu_check_test_23", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "script.nu",
            r#"
                greet "world"

                def greet [name] {
                  echo "hello" $name
                }
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                nu-check -a --as-module script.nu
            "#
        ));

        assert!(actual
            .err
            .contains("You cannot have both `--all` and `--as-module` on the same command line"));
    })
}
