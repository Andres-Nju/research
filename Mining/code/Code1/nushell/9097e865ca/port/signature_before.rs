    fn signature(&self) -> Signature {
        Signature::build("post")
            .optional(
                "start",
                SyntaxShape::Int,
                "The start port to scan (inclusive)",
            )
            .optional("end", SyntaxShape::Int, "The end port to scan (inclusive)")
            .category(Category::Network)
    }
