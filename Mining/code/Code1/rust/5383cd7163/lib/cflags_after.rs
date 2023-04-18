    fn cflags(&self, target: &str) -> Vec<String> {
        // Filter out -O and /O (the optimization flags) that we picked up from
        // gcc-rs because the build scripts will determine that for themselves.
        let mut base = self.cc[target].0.args().iter()
                           .map(|s| s.to_string_lossy().into_owned())
                           .filter(|s| !s.starts_with("-O") && !s.starts_with("/O"))
                           .collect::<Vec<_>>();

        // If we're compiling on OSX then we add a few unconditional flags
        // indicating that we want libc++ (more filled out than libstdc++) and
        // we want to compile for 10.7. This way we can ensure that
        // LLVM/jemalloc/etc are all properly compiled.
        if target.contains("apple-darwin") {
            base.push("-stdlib=libc++".into());
            base.push("-mmacosx-version-min=10.7".into());
        }
        // This is a hack, because newer binutils broke things on some vms/distros
        // (i.e., linking against unknown relocs disabled by the following flag)
        // See: https://github.com/rust-lang/rust/issues/34978
        if target == "x86_64-unknown-linux-musl" {
            base.push("-Wa,-mrelax-relocations=no".into());
        }
        return base
    }
