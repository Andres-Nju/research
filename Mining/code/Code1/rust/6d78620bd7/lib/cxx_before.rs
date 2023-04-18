    fn cxx(&self, target: &str) -> &Path {
        self.cxx[target].path()
    }
