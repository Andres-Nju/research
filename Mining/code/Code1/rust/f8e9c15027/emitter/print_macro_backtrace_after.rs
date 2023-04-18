    fn print_macro_backtrace(&mut self,
                             sp: Span)
                             -> io::Result<()> {
        for trace in self.cm.macro_backtrace(sp) {
            let mut diag_string =
                format!("in this expansion of {}", trace.macro_decl_name);
            if let Some(def_site_span) = trace.def_site_span {
                diag_string.push_str(
                    &format!(" (defined in {})",
                        self.cm.span_to_filename(def_site_span)));
            }
            let snippet = self.cm.span_to_string(trace.call_site);
            print_diagnostic(&mut self.dst, &snippet, Note, &diag_string, None)?;
        }
        Ok(())
    }
