pub fn sort(
    vec: &mut [Value],
    keys: &[Tagged<String>],
    tag: impl Into<Tag>,
    insensitive: bool,
) -> Result<(), ShellError> {
    let tag = tag.into();

    if vec.is_empty() {
        return Err(ShellError::labeled_error(
            "no values to work with",
            "no values to work with",
            tag,
        ));
    }

    for sort_arg in keys.iter() {
        let match_test = &vec[0].get_data_by_key(sort_arg.borrow_spanned());
        if match_test.is_none() {
            return Err(ShellError::labeled_error(
                "Can not find column to sort by",
                "invalid column",
                sort_arg.borrow_spanned().span,
            ));
        }
    }

    match &vec[0] {
        Value {
            value: UntaggedValue::Primitive(_),
            ..
        } => {
            let should_sort_case_insensitively = insensitive && vec.iter().all(|x| x.is_string());

            if let Some(values) = vec
                .windows(2)
                .map(|elem| coerce_compare(&elem[0], &elem[1]))
                .find(|elem| elem.is_err())
            {
                let (type_1, type_2) = values
                    .err()
                    .expect("An error ocourred in the checking of types");
                return Err(ShellError::labeled_error(
                    "Not all values can be compared",
                    format!(
                        "Unable to sort values, as \"{}\" cannot compare against \"{}\"",
                        type_1, type_2
                    ),
                    tag,
                ));
            }

            vec.sort_by(|a, b| {
                if should_sort_case_insensitively {
                    let lowercase_a_string = a.expect_string().to_ascii_lowercase();
                    let lowercase_b_string = b.expect_string().to_ascii_lowercase();

                    lowercase_a_string.cmp(&lowercase_b_string)
                } else {
                    coerce_compare(a, b).expect("Unimplemented BUG: What about primitives that don't have an order defined?").compare()
                }
            });
        }
        _ => {
            let calc_key = |item: &Value| {
                keys.iter()
                    .map(|f| {
                        let mut value_option = item.get_data_by_key(f.borrow_spanned());

                        if insensitive {
                            if let Some(value) = &value_option {
                                if let Ok(string_value) = value.as_string() {
                                    value_option = Some(
                                        UntaggedValue::string(string_value.to_ascii_lowercase())
                                            .into_value(value.tag.clone()),
                                    )
                                }
                            }
                        }

                        value_option
                    })
                    .collect::<Vec<Option<Value>>>()
            };
            vec.sort_by_cached_key(calc_key);
        }
    };

    Ok(())
}
