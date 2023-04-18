    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Get a histogram for the types of files",
                example: "ls | histogram type",
                result: None,
            },
            Example {
                description:
                    "Get a histogram for the types of files, with frequency column named count",
                example: "ls | histogram type count",
                result: None,
            },
            Example {
                description: "Get a histogram for a list of numbers",
                example: "echo [1 2 3 1 1 1 2 2 1 1] | histogram",
                result: None,
            },
        ]
    }
