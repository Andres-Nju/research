    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("watch")
            .required("path", SyntaxShape::Filepath, "the path to watch. Can be a file or directory")
            .required("closure",
            SyntaxShape::Closure(Some(vec![SyntaxShape::String, SyntaxShape::String, SyntaxShape::String])),
                "Some Nu code to run whenever a file changes. The closure will be passed `operation`, `path`, and `new_path` (for renames only) arguments in that order")
            .named(
                "debounce-ms",
                SyntaxShape::Int,
                "Debounce changes for this many milliseconds (default: 100). Adjust if you find that single writes are reported as multiple events",
                Some('d'),
            )
            .named(
                "glob",
                SyntaxShape::String, // SyntaxShape::GlobPattern gets interpreted relative to cwd, so use String instead
                "Only report changes for files that match this glob pattern (default: all files)",
                Some('g'),
            )
            .named(
                "recursive",
                SyntaxShape::Boolean,
                "Watch all directories under `<path>` recursively. Will be ignored if `<path>` is a file (default: true)",
                Some('r'),
            )
            .switch("verbose", "Operate in verbose mode (default: false)", Some('v'))
            .category(Category::FileSystem)
    }
