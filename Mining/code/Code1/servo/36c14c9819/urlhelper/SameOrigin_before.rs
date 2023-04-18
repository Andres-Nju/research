    pub fn SameOrigin(url_a: &ServoUrl, url_b: &ServoUrl) -> bool {
        url_a.origin() == url_b.origin()
    }
