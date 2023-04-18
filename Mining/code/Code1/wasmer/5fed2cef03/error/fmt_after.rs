    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            RuntimeError::Trap { ref msg } => {
                write!(f, "WebAssembly trap occurred during runtime: {}", msg)
            }
            RuntimeError::Error { data } => {
                if let Some(s) = data.downcast_ref::<String>() {
                    write!(f, "\"{}\"", s)
                } else if let Some(s) = data.downcast_ref::<&str>() {
                    write!(f, "\"{}\"", s)
                } else {
                    write!(f, "unknown error")
                }
            }
        }
    }
