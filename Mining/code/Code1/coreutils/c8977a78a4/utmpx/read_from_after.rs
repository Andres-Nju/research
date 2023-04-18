    pub fn read_from(self, f: &str) -> Self {
        let res = unsafe {
            let cstr = CString::new(f).unwrap();
            utmpxname(cstr.as_ptr())
        };
        if res != 0 {
            println!("Warning: {}", IOError::last_os_error());
        }
        unsafe {
            setutxent();
        }
        self
    }
