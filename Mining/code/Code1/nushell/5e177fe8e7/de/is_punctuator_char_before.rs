    fn is_punctuator_char(&mut self, ch: u8) -> bool {
        matches!(ch, b'{' | b'}' | b'[' | b']' | b',' | b':')
    }
