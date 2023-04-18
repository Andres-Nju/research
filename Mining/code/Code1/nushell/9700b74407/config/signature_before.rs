    fn signature(&self) -> Signature {
        Signature::build("config")
            .named(
                "load",
                SyntaxShape::Path,
                "load the config from the path give",
                Some('l'),
            )
            .named(
                "set",
                SyntaxShape::Any,
                "set a value in the config, eg) --set [key value]",
                Some('s'),
            )
            .named(
                "set_into",
                SyntaxShape::String,
                "sets a variable from values in the pipeline",
                Some('i'),
            )
            .named(
                "get",
                SyntaxShape::Any,
                "get a value from the config",
                Some('g'),
            )
            .named(
                "remove",
                SyntaxShape::Any,
                "remove a value from the config",
                Some('r'),
            )
            .switch("clear", "clear the config", Some('c'))
            .switch("path", "return the path to the config file", Some('p'))
    }
