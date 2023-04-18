	fn from(err: std::io::Error) -> Self {
		Error::Io(err.to_string())
	}
