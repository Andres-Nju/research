        fn to_css<W>(&self, dest: &mut W) -> fmt::Result where W: fmt::Write {
            if let Some(angle) = self.angle {
                try!(angle.to_css(dest));
                if self.flipped {
                    dest.write_str(" flip")
                } else {
                    Ok(())
                }
            } else {
                if self.flipped {
                    dest.write_str("flip")
                } else {
                    dest.write_str("from-image")
                }
            }
        }
