    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "List visible files in the current directory",
                example: "ls",
                result: None,
            },
            Example {
                description: "List visible files in a subdirectory",
                example: "ls subdir",
                result: None,
            },
            Example {
                description: "List visible files with full path in the parent directory",
                example: "ls -f ..",
                result: None,
            },
            Example {
                description: "List Rust files",
                example: "ls *.rs",
                result: None,
            },
            Example {
                description: "List files and directories whose name do not contain 'bar'",
                example: "ls -s | where name !~ bar",
                result: None,
            },
            Example {
                description: "List all dirs in your home directory",
                example: "ls -a ~ | where type == dir",
                result: None,
            },
            Example {
                description:
                    "List all dirs in your home directory which have not been modified in 7 days",
                example: "ls -as ~ | where type == dir and modified < ((date now) - 7day)",
                result: None,
            },
            Example {
                description: "List given paths and show directories themselves",
                example: "['/path/to/directory' '/path/to/file'] | each { ls -D $in } | flatten",
                result: None,
            },
        ]
    }
