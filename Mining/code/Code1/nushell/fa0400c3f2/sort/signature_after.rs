    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("sort")
            .switch("reverse", "Sort in reverse order", Some('r'))
            .switch(
                "insensitive",
                "Sort string-based columns case-insensitively",
                Some('i'),
            )
            .category(Category::Filters)
    }
