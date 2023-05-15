    fn shows_error_for_external_command_that_fails() {
        let actual = nu_error!(
            cwd: "tests/fixtures",
            "^false"
        );

        assert!(actual.contains("External command failed"));
    }