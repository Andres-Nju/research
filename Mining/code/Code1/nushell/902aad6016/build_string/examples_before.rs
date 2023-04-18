    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "build-string a b c",
                description: "Builds a string from letters a b c",
                result: Some(Value::String {
                    val: "abc".to_string(),
                    span: Span::test_data(),
                }),
            },
            Example {
                example: r#"build-string $"(1 + 2)" = one ' ' plus ' ' two"#,
                description: "Builds a string from letters a b c",
                result: Some(Value::String {
                    val: "3=one plus two".to_string(),
                    span: Span::test_data(),
                }),
            },
        ]
    }
