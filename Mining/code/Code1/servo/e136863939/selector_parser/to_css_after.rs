    fn to_css<W>(&self, dest: &mut W) -> fmt::Result where W: fmt::Write {
        use cssparser::CssStringWriter;
        use fmt::Write;
        macro_rules! pseudo_class_serialize {
            (bare: [$(($css:expr, $name:ident, $gecko_type:tt, $state:tt, $flags:tt),)*],
             string: [$(($s_css:expr, $s_name:ident, $s_gecko_type:tt, $s_state:tt, $s_flags:tt),)*]) => {
                match *self {
                    $(NonTSPseudoClass::$name => concat!(":", $css),)*
                    $(NonTSPseudoClass::$s_name(ref s) => {
                        write!(dest, ":{}(", $s_css)?;
                        {
                            // FIXME(emilio): Avoid the extra allocation!
                            let mut css = CssStringWriter::new(dest);

                            // Discount the null char in the end from the
                            // string.
                            css.write_str(&String::from_utf16(&s[..s.len() - 1]).unwrap())?;
                        }
                        return dest.write_str(")")
                    }, )*
                    NonTSPseudoClass::MozAny(ref selectors) => {
                        dest.write_str(":-moz-any(")?;
                        let mut iter = selectors.iter();
                        let first = iter.next().expect(":-moz-any must have at least 1 selector");
                        first.to_css(dest)?;
                        for selector in iter {
                            dest.write_str(", ")?;
                            selector.to_css(dest)?;
                        }
                        return dest.write_str(")")
                    }
                }
            }
        }
        let ser = apply_non_ts_list!(pseudo_class_serialize);
        dest.write_str(ser)
    }
