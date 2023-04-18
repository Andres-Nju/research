fn get_output_string(
    headers: &[String],
    rows: &[Vec<String>],
    column_widths: &[usize],
    pretty: bool,
) -> String {
    let mut output_string = String::new();

    if !headers.is_empty() {
        output_string.push('|');

        for i in 0..headers.len() {
            if pretty {
                output_string.push(' ');
                output_string.push_str(&get_padded_string(
                    headers[i].clone(),
                    column_widths[i],
                    ' ',
                ));
                output_string.push(' ');
            } else {
                output_string.push_str(&headers[i]);
            }

            output_string.push('|');
        }

        output_string.push_str("\n|");

        #[allow(clippy::needless_range_loop)]
        for i in 0..headers.len() {
            if pretty {
                output_string.push(' ');
                output_string.push_str(&get_padded_string(
                    String::from("-"),
                    column_widths[i],
                    '-',
                ));
                output_string.push(' ');
            } else {
                output_string.push('-');
            }

            output_string.push('|');
        }

        output_string.push('\n');
    }

    for row in rows {
        if !headers.is_empty() {
            output_string.push('|');
        }

        for i in 0..row.len() {
            if pretty && column_widths.get(i).is_some() {
                output_string.push(' ');
                output_string.push_str(&get_padded_string(row[i].clone(), column_widths[i], ' '));
                output_string.push(' ');
            } else {
                output_string.push_str(&row[i]);
            }

            if !headers.is_empty() {
                output_string.push('|');
            }
        }

        output_string.push('\n');
    }

    output_string
}
