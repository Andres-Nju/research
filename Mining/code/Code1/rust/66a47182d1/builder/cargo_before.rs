    pub fn cargo(
        &self,
        compiler: Compiler,
        mode: Mode,
        target: Interned<String>,
        cmd: &str,
    ) -> Command {
        let mut cargo = Command::new(&self.initial_cargo);
        let out_dir = self.stage_out(compiler, mode);
        cargo
            .env("CARGO_TARGET_DIR", out_dir)
            .arg(cmd);

        if cmd != "install" {
            cargo.arg("--target")
                 .arg(target);
        } else {
            assert_eq!(target, compiler.host);
        }

        // Set a flag for `check` so that certain build scripts can do less work
        // (e.g. not building/requiring LLVM).
        if cmd == "check" {
            cargo.env("RUST_CHECK", "1");
        }

        cargo.arg("-j").arg(self.jobs().to_string());
        // Remove make-related flags to ensure Cargo can correctly set things up
        cargo.env_remove("MAKEFLAGS");
        cargo.env_remove("MFLAGS");

        // FIXME: Temporary fix for https://github.com/rust-lang/cargo/issues/3005
        // Force cargo to output binaries with disambiguating hashes in the name
        let metadata = if compiler.stage == 0 {
            // Treat stage0 like special channel, whether it's a normal prior-
            // release rustc or a local rebuild with the same version, so we
            // never mix these libraries by accident.
            "bootstrap"
        } else {
            &self.config.channel
        };
        cargo.env("__CARGO_DEFAULT_LIB_METADATA", &metadata);

        let stage;
        if compiler.stage == 0 && self.local_rebuild {
            // Assume the local-rebuild rustc already has stage1 features.
            stage = 1;
        } else {
            stage = compiler.stage;
        }

        let mut extra_args = env::var(&format!("RUSTFLAGS_STAGE_{}", stage)).unwrap_or_default();
        if stage != 0 {
            let s = env::var("RUSTFLAGS_STAGE_NOT_0").unwrap_or_default();
            if !extra_args.is_empty() {
                extra_args.push_str(" ");
            }
            extra_args.push_str(&s);
        }

        if !extra_args.is_empty() {
            cargo.env(
                "RUSTFLAGS",
                format!(
                    "{} {}",
                    env::var("RUSTFLAGS").unwrap_or_default(),
                    extra_args
                ),
            );
        }

        let want_rustdoc = self.doc_tests != DocTests::No;

        // We synthetically interpret a stage0 compiler used to build tools as a
        // "raw" compiler in that it's the exact snapshot we download. Normally
        // the stage0 build means it uses libraries build by the stage0
        // compiler, but for tools we just use the precompiled libraries that
        // we've downloaded
        let use_snapshot = mode == Mode::ToolBootstrap;
        assert!(!use_snapshot || stage == 0);

        let maybe_sysroot = self.sysroot(compiler);
        let sysroot = if use_snapshot {
            self.rustc_snapshot_sysroot()
        } else {
            &maybe_sysroot
        };
        let libdir = sysroot.join(libdir(&compiler.host));

        // Customize the compiler we're running. Specify the compiler to cargo
        // as our shim and then pass it some various options used to configure
        // how the actual compiler itself is called.
        //
        // These variables are primarily all read by
        // src/bootstrap/bin/{rustc.rs,rustdoc.rs}
        cargo
            .env("RUSTBUILD_NATIVE_DIR", self.native_dir(target))
            .env("RUSTC", self.out.join("bootstrap/debug/rustc"))
            .env("RUSTC_REAL", self.rustc(compiler))
            .env("RUSTC_STAGE", stage.to_string())
            .env(
                "RUSTC_DEBUG_ASSERTIONS",
                self.config.rust_debug_assertions.to_string(),
            )
            .env("RUSTC_SYSROOT", &sysroot)
            .env("RUSTC_LIBDIR", &libdir)
            .env("RUSTC_RPATH", self.config.rust_rpath.to_string())
            .env("RUSTDOC", self.out.join("bootstrap/debug/rustdoc"))
            .env(
                "RUSTDOC_REAL",
                if cmd == "doc" || (cmd == "test" && want_rustdoc) {
                    self.rustdoc(compiler.host)
                } else {
                    PathBuf::from("/path/to/nowhere/rustdoc/not/required")
                },
            )
            .env("TEST_MIRI", self.config.test_miri.to_string())
            .env("RUSTC_ERROR_METADATA_DST", self.extended_error_dir());

        if let Some(host_linker) = self.linker(compiler.host) {
            cargo.env("RUSTC_HOST_LINKER", host_linker);
        }
        if let Some(target_linker) = self.linker(target) {
            cargo.env("RUSTC_TARGET_LINKER", target_linker);
        }
        if let Some(ref error_format) = self.config.rustc_error_format {
            cargo.env("RUSTC_ERROR_FORMAT", error_format);
        }
        if cmd != "build" && cmd != "check" && want_rustdoc {
            cargo.env("RUSTDOC_LIBDIR", self.sysroot_libdir(compiler, self.config.build));
        }

        if mode.is_tool() {
            // Tools like cargo and rls don't get debuginfo by default right now, but this can be
            // enabled in the config.  Adding debuginfo makes them several times larger.
            if self.config.rust_debuginfo_tools {
                cargo.env("RUSTC_DEBUGINFO", self.config.rust_debuginfo.to_string());
                cargo.env(
                    "RUSTC_DEBUGINFO_LINES",
                    self.config.rust_debuginfo_lines.to_string(),
                );
            }
        } else {
            cargo.env("RUSTC_DEBUGINFO", self.config.rust_debuginfo.to_string());
            cargo.env(
                "RUSTC_DEBUGINFO_LINES",
                self.config.rust_debuginfo_lines.to_string(),
            );
            cargo.env("RUSTC_FORCE_UNSTABLE", "1");

            // Currently the compiler depends on crates from crates.io, and
            // then other crates can depend on the compiler (e.g. proc-macro
            // crates). Let's say, for example that rustc itself depends on the
            // bitflags crate. If an external crate then depends on the
            // bitflags crate as well, we need to make sure they don't
            // conflict, even if they pick the same version of bitflags. We'll
            // want to make sure that e.g. a plugin and rustc each get their
            // own copy of bitflags.

            // Cargo ensures that this works in general through the -C metadata
            // flag. This flag will frob the symbols in the binary to make sure
            // they're different, even though the source code is the exact
            // same. To solve this problem for the compiler we extend Cargo's
            // already-passed -C metadata flag with our own. Our rustc.rs
            // wrapper around the actual rustc will detect -C metadata being
            // passed and frob it with this extra string we're passing in.
            cargo.env("RUSTC_METADATA_SUFFIX", "rustc");
        }

        if let Some(x) = self.crt_static(target) {
            cargo.env("RUSTC_CRT_STATIC", x.to_string());
        }

        if let Some(x) = self.crt_static(compiler.host) {
            cargo.env("RUSTC_HOST_CRT_STATIC", x.to_string());
        }

        // Enable usage of unstable features
        cargo.env("RUSTC_BOOTSTRAP", "1");
        self.add_rust_test_threads(&mut cargo);

        // Almost all of the crates that we compile as part of the bootstrap may
        // have a build script, including the standard library. To compile a
        // build script, however, it itself needs a standard library! This
        // introduces a bit of a pickle when we're compiling the standard
        // library itself.
        //
        // To work around this we actually end up using the snapshot compiler
        // (stage0) for compiling build scripts of the standard library itself.
        // The stage0 compiler is guaranteed to have a libstd available for use.
        //
        // For other crates, however, we know that we've already got a standard
        // library up and running, so we can use the normal compiler to compile
        // build scripts in that situation.
        //
        // If LLVM support is disabled we need to use the snapshot compiler to compile
        // build scripts, as the new compiler doesn't support executables.
        if mode == Mode::Std || !self.config.llvm_enabled {
            cargo
                .env("RUSTC_SNAPSHOT", &self.initial_rustc)
                .env("RUSTC_SNAPSHOT_LIBDIR", self.rustc_snapshot_libdir());
        } else {
            cargo
                .env("RUSTC_SNAPSHOT", self.rustc(compiler))
                .env("RUSTC_SNAPSHOT_LIBDIR", self.rustc_libdir(compiler));
        }

        if self.config.incremental {
            cargo.env("CARGO_INCREMENTAL", "1");
        }

        if let Some(ref on_fail) = self.config.on_fail {
            cargo.env("RUSTC_ON_FAIL", on_fail);
        }

        if self.config.print_step_timings {
            cargo.env("RUSTC_PRINT_STEP_TIMINGS", "1");
        }

        if self.config.backtrace_on_ice {
            cargo.env("RUSTC_BACKTRACE_ON_ICE", "1");
        }

        if self.config.rust_verify_llvm_ir {
            cargo.env("RUSTC_VERIFY_LLVM_IR", "1");
        }

        cargo.env("RUSTC_VERBOSE", self.verbosity.to_string());

        // in std, we want to avoid denying warnings for stage 0 as that makes cfg's painful.
        if self.config.deny_warnings && !(mode == Mode::Std && stage == 0) {
            cargo.env("RUSTC_DENY_WARNINGS", "1");
        }

        // Throughout the build Cargo can execute a number of build scripts
        // compiling C/C++ code and we need to pass compilers, archivers, flags, etc
        // obtained previously to those build scripts.
        // Build scripts use either the `cc` crate or `configure/make` so we pass
        // the options through environment variables that are fetched and understood by both.
        //
        // FIXME: the guard against msvc shouldn't need to be here
        if target.contains("msvc") {
            if let Some(ref cl) = self.config.llvm_clang_cl {
                cargo.env("CC", cl).env("CXX", cl);
            }
        } else {
            let ccache = self.config.ccache.as_ref();
            let ccacheify = |s: &Path| {
                let ccache = match ccache {
                    Some(ref s) => s,
                    None => return s.display().to_string(),
                };
                // FIXME: the cc-rs crate only recognizes the literal strings
                // `ccache` and `sccache` when doing caching compilations, so we
                // mirror that here. It should probably be fixed upstream to
                // accept a new env var or otherwise work with custom ccache
                // vars.
                match &ccache[..] {
                    "ccache" | "sccache" => format!("{} {}", ccache, s.display()),
                    _ => s.display().to_string(),
                }
            };
            let cc = ccacheify(&self.cc(target));
            cargo.env(format!("CC_{}", target), &cc).env("CC", &cc);

            let cflags = self.cflags(target).join(" ");
            cargo
                .env(format!("CFLAGS_{}", target), cflags.clone())
                .env("CFLAGS", cflags.clone());

            if let Some(ar) = self.ar(target) {
                let ranlib = format!("{} s", ar.display());
                cargo
                    .env(format!("AR_{}", target), ar)
                    .env("AR", ar)
                    .env(format!("RANLIB_{}", target), ranlib.clone())
                    .env("RANLIB", ranlib);
            }

            if let Ok(cxx) = self.cxx(target) {
                let cxx = ccacheify(&cxx);
                cargo
                    .env(format!("CXX_{}", target), &cxx)
                    .env("CXX", &cxx)
                    .env(format!("CXXFLAGS_{}", target), cflags.clone())
                    .env("CXXFLAGS", cflags);
            }
        }

        if cmd == "build"
            && mode == Mode::Std
            && self.config.extended
            && compiler.is_final_stage(self)
        {
            cargo.env("RUSTC_SAVE_ANALYSIS", "api".to_string());
        }

        // For `cargo doc` invocations, make rustdoc print the Rust version into the docs
        cargo.env("RUSTDOC_CRATE_VERSION", self.rust_version());

        // Environment variables *required* throughout the build
        //
        // FIXME: should update code to not require this env var
        cargo.env("CFG_COMPILER_HOST_TRIPLE", target);

        // Set this for all builds to make sure doc builds also get it.
        cargo.env("CFG_RELEASE_CHANNEL", &self.config.channel);

        // This one's a bit tricky. As of the time of this writing the compiler
        // links to the `winapi` crate on crates.io. This crate provides raw
        // bindings to Windows system functions, sort of like libc does for
        // Unix. This crate also, however, provides "import libraries" for the
        // MinGW targets. There's an import library per dll in the windows
        // distribution which is what's linked to. These custom import libraries
        // are used because the winapi crate can reference Windows functions not
        // present in the MinGW import libraries.
        //
        // For example MinGW may ship libdbghelp.a, but it may not have
        // references to all the functions in the dbghelp dll. Instead the
        // custom import library for dbghelp in the winapi crates has all this
        // information.
        //
        // Unfortunately for us though the import libraries are linked by
        // default via `-ldylib=winapi_foo`. That is, they're linked with the
        // `dylib` type with a `winapi_` prefix (so the winapi ones don't
        // conflict with the system MinGW ones). This consequently means that
        // the binaries we ship of things like rustc_codegen_llvm (aka the rustc_codegen_llvm
        // DLL) when linked against *again*, for example with procedural macros
        // or plugins, will trigger the propagation logic of `-ldylib`, passing
        // `-lwinapi_foo` to the linker again. This isn't actually available in
        // our distribution, however, so the link fails.
        //
        // To solve this problem we tell winapi to not use its bundled import
        // libraries. This means that it will link to the system MinGW import
        // libraries by default, and the `-ldylib=foo` directives will still get
        // passed to the final linker, but they'll look like `-lfoo` which can
        // be resolved because MinGW has the import library. The downside is we
        // don't get newer functions from Windows, but we don't use any of them
        // anyway.
        if !mode.is_tool() {
            cargo.env("WINAPI_NO_BUNDLED_LIBRARIES", "1");
        }

        for _ in 1..self.verbosity {
            cargo.arg("-v");
        }

        // This must be kept before the thinlto check, as we set codegen units
        // to 1 forcibly there.
        if let Some(n) = self.config.rust_codegen_units {
            cargo.env("RUSTC_CODEGEN_UNITS", n.to_string());
        }

        if self.config.rust_optimize {
            // FIXME: cargo bench/install do not accept `--release`
            if cmd != "bench" && cmd != "install" {
                cargo.arg("--release");
            }
        }

        if self.config.locked_deps {
            cargo.arg("--locked");
        }
        if self.config.vendor || self.is_sudo {
            cargo.arg("--frozen");
        }

        self.ci_env.force_coloring_in_ci(&mut cargo);

        cargo
    }
