    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "show all commands and sub-commands",
                example: "help commands",
                result: None,
            },
            Example {
                description: "generate documentation",
                example: "help generate_docs",
                result: None,
            },
            Example {
                description: "show help for single command",
                example: "help match",
                result: None,
            },
            Example {
                description: "show help for single sub-command",
                example: "help str lpad",
                result: None,
            },
            Example {
                description: "search for string in command names, usage and search terms",
                example: "help --find char",
                result: None,
            },
        ]
    }
