    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "get free port between 3121 and 4000",
                example: "port 3121 4000",
                result: Some(Value::Int {
                    val: 3121,
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "get free port from system",
                example: "port",
                result: None,
            },
        ]
    }
