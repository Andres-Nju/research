fn validate_ascii_code_escape(text: &str, range: TextRange, errors: &mut Vec<SyntaxError>) {
    // An AsciiCodeEscape has 4 chars, example: `\xDD`
    if !text.is_ascii() {
        // TODO: Give a more precise error message (say what the invalid character was)
        errors.push(SyntaxError::new(AsciiCodeEscapeOutOfRange, range));
    }
    if text.len() < 4 {
        errors.push(SyntaxError::new(TooShortAsciiCodeEscape, range));
    } else {
        assert_eq!(
            text.len(),
            4,
            "AsciiCodeEscape cannot be longer than 4 chars, but text '{}' is",
            text,
        );

        match u8::from_str_radix(&text[2..], 16) {
            Ok(code) if code < 128 => { /* Escape code is valid */ }
            Ok(_) => errors.push(SyntaxError::new(AsciiCodeEscapeOutOfRange, range)),
            Err(_) => errors.push(SyntaxError::new(MalformedAsciiCodeEscape, range)),
        }
    }
}
