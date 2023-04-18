        fn to_css<W>(&self, dest: &mut W) -> fmt::Result where W: fmt::Write {
            match *self {
                computed_value::T::FromImage => dest.write_str("from-image"),
                computed_value::T::AngleWithFlipped(angle, flipped) => {
                    try!(angle.to_css(dest));
                    if flipped {
                        try!(dest.write_str(" flip"));
                    }
                    Ok(())
                },
            }
        }
