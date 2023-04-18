    fn write_text(&mut self, text: &str) {
      // windows psuedo console requires a \r\n to do a newline
      let newline_re = regex::Regex::new("\r?\n").unwrap();
      self
        .write_all(newline_re.replace_all(text, "\r\n").as_bytes())
        .unwrap();
    }
