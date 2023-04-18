    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("merge")
            .input_output_types(vec![
                (Type::Record(vec![]), Type::Record(vec![])),
                (Type::Table(vec![]), Type::Table(vec![])),
            ])
            .required(
                "value",
                // Both this and `update` should have a shape more like <record> | <table> than just <any>. -Leon 2022-10-27
                SyntaxShape::Any,
                "the new value to merge with",
            )
            .category(Category::Filters)
    }
