    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Returns boolean indicating if pattern was found",
            example: "[abc acb acb] | dataframe to-df | dataframe contains ab",
            result: Some(vec![NuDataFrame::try_from_columns(
                vec![Column::new(
                    "0".to_string(),
                    vec![
                        UntaggedValue::boolean(true).into(),
                        UntaggedValue::boolean(false).into(),
                        UntaggedValue::boolean(false).into(),
                    ],
                )],
                &Span::default(),
            )
            .expect("simple df for test should not fail")
            .into_value(Tag::default())]),
        }]
    }
