    fn filename(&self, component: &str, target: &str) -> String {
        if component == "rust-src" {
            format!("rust-src-{}.tar.gz", self.rust_release)
        } else if component == "cargo" {
            format!("cargo-{}-{}.tar.gz", self.cargo_release, target)
        } else if component == "rls" || component == "rls-preview" {
            format!("rls-{}-{}.tar.gz", self.rls_release, target)
        } else if component == "clippy" || component == "clippy-preview" {
            format!("clippy-{}-{}.tar.gz", self.clippy_release, target)
        } else if component == "rustfmt" || component == "rustfmt-preview" {
            format!("rustfmt-{}-{}.tar.gz", self.rustfmt_release, target)
        } else if component == "llvm_tools" {
            format!("llvm-tools-{}-{}.tar.gz", self.llvm_tools_release, target)
        } else {
            format!("{}-{}-{}.tar.gz", component, self.rust_release, target)
        }
    }
