    fn to_css<W>(&self, dest: &mut W) -> fmt::Result where W: fmt::Write {
        dest.write_str("circle(")?;
        self.radius.to_css(dest)?;
        dest.write_str(" at ")?;
        self.position.to_css(dest)
    }
