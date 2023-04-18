    fn cxx(&self, target: &str) -> &Path {
        match self.cxx.get(target) {
            Some(p) => p.path(),
            None => panic!("\n\ntarget `{}` is not configured as a host,
                            only as a target\n\n", target),
        }
    }
