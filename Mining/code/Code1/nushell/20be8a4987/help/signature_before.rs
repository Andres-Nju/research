    fn signature(&self) -> Signature {
        Signature::build("help")
            .rest(
                "rest",
                SyntaxShape::String,
                "the name of command to get help on",
            )
            .named(
                "find",
                SyntaxShape::String,
                "string to find in command usage",
                Some('f'),
            )
            .category(Category::Core)
    }
