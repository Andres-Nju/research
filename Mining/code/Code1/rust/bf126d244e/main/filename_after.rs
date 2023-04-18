    fn filename(&self, component: &str, target: &str) -> String {
        if component == "rust-src" {
            format!("rust-src-{}.tar.gz", self.channel)
        } else if component == "cargo" {
            format!("cargo-nightly-{}.tar.gz", target)
        } else {
            format!("{}-{}-{}.tar.gz", component, self.channel, target)
        }
    }
