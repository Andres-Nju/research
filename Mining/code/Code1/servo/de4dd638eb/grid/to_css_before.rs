    fn to_css<W>(&self, dest: &mut W) -> fmt::Result where W: fmt::Write {
        if self.is_auto() {
            return dest.write_str("auto")
        }

        if self.is_span {
            dest.write_str("span")?;
        }

        if let Some(ref i) = self.line_num {
            dest.write_str(" ")?;
            i.to_css(dest)?;
        }

        if let Some(ref s) = self.ident {
            dest.write_str(" ")?;
            s.to_css(dest)?;
        }

        Ok(())
    }