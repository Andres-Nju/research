    pub fn Origin(url: &ServoUrl) -> USVString {
        USVString(quirks::origin(url.as_url().unwrap()).to_owned())
    }
