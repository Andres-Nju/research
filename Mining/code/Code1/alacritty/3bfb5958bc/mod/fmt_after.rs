    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Error::MissingGlyph(c) => write!(f, "Glyph not found for char {:?}", c),
            Error::MissingFont(desc) => write!(
                f,
                "Couldn't find a font with {}\n\tPlease check the font config in your \
                 alacritty.yml.",
                desc
            ),
            Error::FontNotLoaded => f.write_str("Tried to use a font that hasn't been loaded"),
            Error::DirectWriteError(hresult) => {
                write!(f, "A DirectWrite rendering error occurred: {:#X}", hresult)
            },
        }
    }
