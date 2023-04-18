    fn signature(&self) -> Signature {
        Signature::build("pivot")
            .switch(
                "header-row",
                "treat the first row as column names",
                Some('h'),
            )
            .switch(
                "ignore-titles",
                "don't pivot the column names into values",
                Some('i'),
            )
            .rest(
                SyntaxShape::String,
                "the names to give columns once pivoted",
            )
    }
