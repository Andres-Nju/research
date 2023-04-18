    pub fn new(str: &str) -> InternedString {
        let mut cache = STRING_CACHE.write().unwrap();
        if let Some(&s) = cache.get(str) {
            return InternedString {
                ptr: s.as_ptr(),
                len: s.len(),
            };
        }
        let s = leak(str.to_string());
        cache.insert(s);
        InternedString {
            ptr: s.as_ptr(),
            len: s.len(),
        }
    }
