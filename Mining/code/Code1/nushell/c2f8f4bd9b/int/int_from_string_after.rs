fn int_from_string(a_string: &str, span: Span) -> Result<i64, ShellError> {
    let trimmed = a_string.trim();
    match trimmed {
        b if b.starts_with("0b") => {
            let num = match i64::from_str_radix(b.trim_start_matches("0b"), 2) {
                Ok(n) => n,
                Err(_reason) => {
                    return Err(ShellError::CantConvert(
                        "int".to_string(),
                        "string".to_string(),
                        span,
                        Some(r#"digits following "0b" can only be 0 or 1"#.to_string()),
                    ))
                }
            };
            Ok(num)
        }
        h if h.starts_with("0x") => {
            let num =
                match i64::from_str_radix(h.trim_start_matches("0x"), 16) {
                    Ok(n) => n,
                    Err(_reason) => return Err(ShellError::CantConvert(
                        "int".to_string(),
                        "string".to_string(),
                        span,
                        Some(
                            r#"hexadecimal digits following "0x" should be in 0-9, a-f, or A-F"#
                                .to_string(),
                        ),
                    )),
                };
            Ok(num)
        }
        _ => match trimmed.parse::<i64>() {
            Ok(n) => Ok(n),
            Err(_) => match a_string.parse::<f64>() {
                Ok(f) => Ok(f as i64),
                _ => Err(ShellError::CantConvert(
                    "int".to_string(),
                    "string".to_string(),
                    span,
                    Some(format!(
                        r#"string "{}" does not represent a valid integer"#,
                        trimmed
                    )),
                )),
            },
        },
    }
}
