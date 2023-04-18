pub fn action(input: &Value, span: Span) -> Value {
    if let Ok(value_span) = input.span() {
        match input {
            Value::Filesize { .. } => input.clone(),
            Value::Int { val, .. } => Value::Filesize {
                val: *val,
                span: value_span,
            },
            Value::Float { val, .. } => Value::Filesize {
                val: *val as i64,
                span: value_span,
            },
            Value::String { val, .. } => match int_from_string(val, value_span) {
                Ok(val) => Value::Filesize {
                    val,
                    span: value_span,
                },
                Err(error) => Value::Error { error },
            },
            _ => Value::Error {
                error: ShellError::UnsupportedInput(
                    "'into filesize' for unsupported type".into(),
                    value_span,
                ),
            },
        }
    } else {
        Value::Error {
            error: ShellError::UnsupportedInput(
                "'into filesize' for unsupported type".into(),
                span,
            ),
        }
    }
}
