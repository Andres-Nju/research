    fn examples(&self) -> &[Example] {
        &[
            Example {
                description: "Get the second row",
                example: "echo [first second third] | get 1",
            },
            Example {
                description: "Get the first and third rows",
                example: "echo [first second third] | get 0 2",
            },
        ]
    }
