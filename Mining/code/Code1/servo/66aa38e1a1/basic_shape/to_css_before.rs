    fn to_css<W>(&self, dest: &mut W) -> fmt::Result where W: fmt::Write {
        self.radius.to_css(dest)?;
        dest.write_str(" at ")?;
        self.position.to_css(dest)
    }
