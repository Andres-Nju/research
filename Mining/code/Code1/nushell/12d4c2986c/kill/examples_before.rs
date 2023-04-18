    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Kill the pid using the most memory",
                example: "ps | sort-by mem | last | kill $it.pid",
                result: None,
            },
            Example {
                description: "Force kill a given pid",
                example: "kill --force 12345",
                result: None,
            },
            Example {
                description: "Send INT signal",
                example: "kill -s 2 12345",
                result: None,
            },
        ]
    }
