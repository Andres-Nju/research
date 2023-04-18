    fn examples(&self) -> Vec<Example> {
        let mut examples = vec![Example {
            description:
                "Delete, or move a file to the trash (based on the 'always_trash' config option)",
            example: "rm file.txt",
            result: None,
        }];
        #[cfg(all(
            feature = "trash-support",
            not(target_os = "android"),
            not(target_os = "ios")
        ))]
        examples.append(&mut vec![
            Example {
                description: "Move a file to the trash",
                example: "rm --trash file.txt",
                result: None,
            },
            Example {
                description:
                    "Delete a file permanently, even if the 'always_trash' config option is true",
                example: "rm --permanent file.txt",
                result: None,
            },
        ]);
        examples.push(Example {
            description: "Delete a file, ignoring 'file not found' errors",
            example: "rm --force file.txt",
            result: None,
        });
        examples.push(Example {
            description: "Delete all 0KB files in the current directory",
            example: "ls | where size == 0KB && type == file | each { rm $in.name } | null",
            result: None,
        });
        examples
    }
