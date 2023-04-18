    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "echo done | ignore",
            example: r#"echo "There are seven words in this sentence" | size"#,
            result: None,
        }]
    }
