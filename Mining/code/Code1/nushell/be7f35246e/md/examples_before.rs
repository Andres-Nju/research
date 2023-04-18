    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Outputs an MD string representing the contents of this table",
                example: "[[foo bar]; [1 2]] | to md",
                result: Some(Value::test_string("|foo|bar|\n|-|-|\n|1|2|\n")),
            },
            Example {
                description: "Optionally, output a formatted markdown string",
                example: "[[foo bar]; [1 2]] | to md --pretty",
                result: Some(Value::test_string(
                    "| foo | bar |\n| --- | --- |\n| 1   | 2   |\n",
                )),
            },
            Example {
                description: "Treat each row as a markdown element",
                example: r#"[{"H1": "Welcome to Nushell" } [[foo bar]; [1 2]]] | to md --per-element --pretty"#,
                result: Some(Value::test_string(
                    "# Welcome to Nushell\n| foo | bar |\n| --- | --- |\n| 1   | 2   |",
                )),
            },
        ]
    }
