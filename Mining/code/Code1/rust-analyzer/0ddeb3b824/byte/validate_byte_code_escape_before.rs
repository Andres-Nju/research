fn validate_byte_code_escape(text: &str, range: TextRange, errors: &mut Vec<SyntaxError>) {
    // A ByteCodeEscape has 4 chars, example: `\xDD`
    if text.len() < 4 {
        errors.push(SyntaxError::new(TooShortByteCodeEscape, range));
    } else {
        assert!(
            text.chars().count() == 4,
            "ByteCodeEscape cannot be longer than 4 chars"
        );

        if u8::from_str_radix(&text[2..], 16).is_err() {
            errors.push(SyntaxError::new(MalformedByteCodeEscape, range));
        }
    }
}
