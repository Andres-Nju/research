    pub fn read_from(self, f: &str) -> Self {
        // FixME: discuss and revise a rewrite which is correct and satisfies clippy/rustc
        #[allow(clippy::temporary_cstring_as_ptr)]
        let res = unsafe { utmpxname(CString::new(f).unwrap().as_ptr()) };
        if res != 0 {
            println!("Warning: {}", IOError::last_os_error());
        }
        unsafe {
            setutxent();
        }
        self
    }
