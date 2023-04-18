        fn to_css<W>(&self, dest: &mut W) -> fmt::Result where W: fmt::Write {
            try!(self.0.to_css(dest));
            try!(dest.write_str(" "));
            self.0.to_css(dest)
        }
