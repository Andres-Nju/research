    fn debuginfo_map(&self, which: GitRepo) -> Option<String> {
        if !self.config.rust_remap_debuginfo {
            return None
        }

        let path = match which {
            GitRepo::Rustc => {
                let sha = self.rust_sha().unwrap_or(channel::CFG_RELEASE_NUM);
                format!("/rustc/{}", sha)
            }
            GitRepo::Llvm => format!("/rustc/llvm"),
        };
        Some(format!("{}={}", self.src.display(), path))
    }
