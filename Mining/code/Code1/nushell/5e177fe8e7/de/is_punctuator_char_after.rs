    fn is_punctuator_char(&self, ch: u8) -> bool {
        matches!(ch, b'{' | b'}' | b'[' | b']' | b',' | b':')
    }
