    pub fn cwd<T: AsRef<OsStr>>(&mut self, path: T) -> &mut Self {
        if let Some(ref mut p) = self.process_builder {
            if let Some(cwd) = p.get_cwd() {
                let new_path = cwd.join(path.as_ref());
                p.cwd(new_path);
            } else {
                p.cwd(path);
            }
        }
        self
    }
