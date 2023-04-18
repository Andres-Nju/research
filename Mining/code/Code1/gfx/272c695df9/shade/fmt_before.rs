    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            CreateShaderError::ModelNotSupported |
            CreateShaderError::LibrarySourceNotSupported |
            CreateShaderError::CompilationFailed(_) => f.pad(self.description()),
            CreateShaderError::StageNotSupported(ref stage) => {
                write!("the device does not support the {:?} stage", stage)
            }
        }
    }
