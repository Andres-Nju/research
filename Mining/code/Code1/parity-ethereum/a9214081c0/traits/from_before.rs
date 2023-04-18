	fn from(err: std::io::Error) -> Self {
		Error::Io(err.description().to_owned())
	}
