        fn to_css<W>(&self, dest: &mut W) -> fmt::Result where W: fmt::Write {
            if let Some(angle) = self.angle {
                try!(angle.to_css(dest));
                if self.flipped {
                    dest.write_str(" flipped")
                } else {
                    Ok(())
                }
            } else {
                if self.flipped {
                    dest.write_str("flipped")
                } else {
                    dest.write_str("from-image")
                }
            }
        }
