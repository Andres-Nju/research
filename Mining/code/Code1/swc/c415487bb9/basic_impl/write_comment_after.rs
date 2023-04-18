    fn write_comment(&mut self, span: Span, s: &str) -> Result {
        self.write(Some(span), s)?;
        {
            let line_start_of_s = compute_line_starts(s);
            if line_start_of_s.len() > 1 {
                self.line_count = self.line_count + line_start_of_s.len() - 1;
                self.line_pos = s.len() - line_start_of_s.last().cloned().unwrap_or(0);
            }
        }
        Ok(())
    }
