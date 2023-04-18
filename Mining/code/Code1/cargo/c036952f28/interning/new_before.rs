    pub fn new(str: &str) -> InternedString {
        let mut cache = STRING_CASHE.write().unwrap();
        if let Some(&s) = cache.get(str) {
            return InternedString {
                ptr: s.as_ptr(),
                len: s.len(),
            };
        }
        let s = leek(str.to_string());
        cache.insert(s);
        InternedString {
            ptr: s.as_ptr(),
            len: s.len(),
        }
    }
