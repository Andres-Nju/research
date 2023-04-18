    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Ignore the output of an echo command",
            example: r#"echo done | ignore"#,
            result: None,
        }]
    }
