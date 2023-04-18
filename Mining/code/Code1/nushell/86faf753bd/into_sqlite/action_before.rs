fn action(
    input: &Value,
    table: Option<Spanned<String>>,
    file: Spanned<String>,
    span: Span,
) -> Result<Value, ShellError> {
    let table_name = if let Some(table_name) = table {
        table_name.item
    } else {
        "main".to_string()
    };

    match input {
        Value::List { vals, span } => {
            // find the column names, and sqlite data types
            let columns = get_columns_with_sqlite_types(vals);

            let table_columns_creation = columns
                .iter()
                .map(|(name, sql_type)| format!("{name} {sql_type}"))
                .join(",");

            // get the values
            let table_values = vals
                .iter()
                .map(|list_value| {
                    format!(
                        "({})",
                        match list_value {
                            Value::Record {
                                cols: _,
                                vals,
                                span: _,
                            } => {
                                vals.iter()
                                    .map(|rec_val| {
                                        format!("'{}'", nu_value_to_string(rec_val.clone(), ""))
                                    })
                                    .join(",")
                            }
                            // Number formats so keep them without quotes
                            Value::Int { val: _, span: _ }
                            | Value::Float { val: _, span: _ }
                            | Value::Filesize { val: _, span: _ }
                            | Value::Duration { val: _, span: _ } =>
                                nu_value_to_string(list_value.clone(), ""),
                            _ =>
                            // String formats so add quotes around them
                                format!("'{}'", nu_value_to_string(list_value.clone(), "")),
                        }
                    )
                })
                .join(",");

            // create the sqlite database table
            let conn = open_sqlite_db(Path::new(&file.item), file.span)?;

            // create a string for sql table creation
            let create_statement =
                format!("CREATE TABLE IF NOT EXISTS {table_name} ({table_columns_creation})");

            // prepare the string as a sqlite statement
            let mut stmt = conn.prepare(&create_statement).map_err(|e| {
                ShellError::GenericError(
                    "Failed to prepare SQLite statement".into(),
                    e.to_string(),
                    Some(file.span),
                    None,
                    Vec::new(),
                )
            })?;

            // execute the statement
            stmt.execute([]).map_err(|e| {
                ShellError::GenericError(
                    "Failed to execute SQLite statement".into(),
                    e.to_string(),
                    Some(file.span),
                    None,
                    Vec::new(),
                )
            })?;

            // use normal sql to create the table
            // insert into table_name
            // values
            // ('xx', 'yy', 'zz'),
            // ('aa', 'bb', 'cc'),
            // ('dd', 'ee', 'ff')

            // create the string for inserting data into the table
            let insert_statement = format!("INSERT INTO {table_name} VALUES {table_values}");

            // prepare the string as a sqlite statement
            let mut stmt = conn.prepare(&insert_statement).map_err(|e| {
                ShellError::GenericError(
                    "Failed to prepare SQLite statement".into(),
                    e.to_string(),
                    Some(file.span),
                    None,
                    Vec::new(),
                )
            })?;

            // execute the statement
            stmt.execute([]).map_err(|e| {
                ShellError::GenericError(
                    "Failed to execute SQLite statement".into(),
                    e.to_string(),
                    Some(file.span),
                    None,
                    Vec::new(),
                )
            })?;

            // and we're done
            Ok(Value::Nothing { span: *span })
        }
        // Propagate errors by explicitly matching them before the final case.
        Value::Error { error } => Err(*error.clone()),
        other => Err(ShellError::OnlySupportsThisInputType {
            exp_input_type: "list".into(),
            wrong_type: other.get_type().to_string(),
            dst_span: span,
            src_span: other.expect_span(),
        }),
    }
}
