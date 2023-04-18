    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::ShaderCreation(err) => err.source(),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Error::ShaderCreation(err) => {
                write!(f, "There was an error initializing the shaders: {}", err)
            },
        }
    }
