    fn signature(&self) -> Signature {
        Signature::build("path join")
            .rest(SyntaxShape::ColumnPath, "Optionally operate by column path")
            .named(
                "append",
                SyntaxShape::String,
                "Path to append to the input",
                Some('a'),
            )
    }
