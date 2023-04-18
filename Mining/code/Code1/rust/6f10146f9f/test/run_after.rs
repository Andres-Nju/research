    fn run(self, builder: &Builder) {
        let rustdoc = builder.out.join("bootstrap/debug/rustdoc");
        let mut cmd = builder.tool_cmd(Tool::RustdocTheme);
        cmd.arg(rustdoc.to_str().unwrap())
           .arg(builder.src.join("src/librustdoc/html/static/themes").to_str().unwrap())
           .env("RUSTC_STAGE", self.compiler.stage.to_string())
           .env("RUSTC_SYSROOT", builder.sysroot(self.compiler))
           .env("RUSTDOC_LIBDIR", builder.sysroot_libdir(self.compiler, self.compiler.host))
           .env("CFG_RELEASE_CHANNEL", &builder.build.config.channel)
           .env("RUSTDOC_REAL", builder.rustdoc(self.compiler.host))
           .env("RUSTDOC_CRATE_VERSION", builder.build.rust_version())
           .env("RUSTC_BOOTSTRAP", "1");
        if let Some(linker) = builder.build.linker(self.compiler.host) {
            cmd.env("RUSTC_TARGET_LINKER", linker);
        }
        try_run(builder.build, &mut cmd);
    }
