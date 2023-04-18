    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Set the MYENV environment variable",
                example: r#"with-env [MYENV "my env value"] { $env.MYENV }"#,
                result: Some(Value::test_string("my env value")),
            },
            Example {
                description: "Set by primitive value list",
                example: r#"with-env [X Y W Z] { $env.X }"#,
                result: Some(Value::test_string("Y")),
            },
            Example {
                description: "Set by single row table",
                example: r#"with-env [[X W]; [Y Z]] { $env.W }"#,
                result: Some(Value::test_string("Z")),
            },
            Example {
                description: "Set by row(e.g. `open x.json` or `from json`)",
                example: r#"echo '{"X":"Y","W":"Z"}'|from json|with-env $it { echo $env.X $env.W }"#,
                result: None,
            },
        ]
    }
