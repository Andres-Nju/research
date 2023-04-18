fn table(input: PipelineData, pretty: bool, config: &Config) -> String {
    let vec_of_values = input.into_iter().collect::<Vec<Value>>();
    let headers = merge_descriptors(&vec_of_values);

    let (escaped_headers, mut column_widths) = collect_headers(&headers);

    let mut escaped_rows: Vec<Vec<String>> = Vec::new();

    for row in vec_of_values {
        let mut escaped_row: Vec<String> = Vec::new();

        match row.to_owned() {
            Value::Record { span, .. } => {
                for i in 0..headers.len() {
                    let data = row.get_data_by_key(&headers[i]);
                    let value_string = data
                        .unwrap_or_else(|| Value::nothing(span))
                        .into_string(", ", config);
                    let new_column_width = value_string.len();

                    escaped_row.push(value_string);

                    if column_widths[i] < new_column_width {
                        column_widths[i] = new_column_width;
                    }
                }
            }
            p => {
                let value_string = htmlescape::encode_minimal(&p.into_abbreviated_string(config));
                escaped_row.push(value_string);
            }
        }

        escaped_rows.push(escaped_row);
    }

    let output_string = if (column_widths.is_empty() || column_widths.iter().all(|x| *x == 0))
        && escaped_rows.is_empty()
    {
        String::from("")
    } else {
        get_output_string(&escaped_headers, &escaped_rows, &column_widths, pretty)
            .trim()
            .to_string()
    };

    output_string
}
