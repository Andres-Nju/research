fn _to_timezone(dt: DateTime<FixedOffset>, timezone: &Spanned<String>, span: Span) -> Value {
    match datetime_in_timezone(&dt, timezone.item.as_str()) {
        Ok(dt) => Value::Date { val: dt, span },
        Err(_) => Value::Error {
            error: ShellError::UnsupportedInput(String::from("invalid time zone"), span),
        },
    }
}
