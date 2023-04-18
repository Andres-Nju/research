    pub fn cwd<T: AsRef<OsStr>>(&mut self, path: T) -> &mut Self {
        if let Some(ref mut p) = self.process_builder {
            if let Some(cwd) = p.get_cwd() {
                p.cwd(cwd.join(path.as_ref()));
            } else {
                p.cwd(path);
            }
        }
        self
    }
