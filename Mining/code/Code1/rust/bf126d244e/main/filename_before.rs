    fn filename(&self, component: &str, target: &str) -> String {
        if component == "rust-src" {
            format!("rust-src-{}.tar.gz", self.channel)
        } else {
            format!("{}-{}-{}.tar.gz", component, self.channel, target)
        }
    }
