    pub fn new(sess: &'a ParseSess,
               source_file: Lrc<syntax_pos::SourceFile>,
               override_span: Option<Span>) -> Self {
        let mut sr = StringReader::new_raw(sess, source_file, override_span);
        if sr.advance_token().is_err() {
            sr.emit_fatal_errors();
            FatalError.raise();
        }

        sr
    }

    pub fn new_without_err(sess: &'a ParseSess,
                           source_file: Lrc<syntax_pos::SourceFile>,
                           override_span: Option<Span>) -> Result<Self, ()> {
        let mut sr = StringReader::new_raw(sess, source_file, override_span);
        if sr.advance_token().is_err() {
            sr.emit_fatal_errors();
            return Err(());
        }
        Ok(sr)
    }
