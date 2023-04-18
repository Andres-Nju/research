    pub fn parse(args: &[String]) -> Config {
        let flags = Flags::parse(&args);
        let file = flags.config.clone();
        let mut config = Config::default_opts();
        config.exclude = flags.exclude;
        config.rustc_error_format = flags.rustc_error_format;
        config.on_fail = flags.on_fail;
        config.stage = flags.stage;
        config.jobs = flags.jobs;
        config.cmd = flags.cmd;
        config.incremental = flags.incremental;
        config.dry_run = flags.dry_run;
        config.keep_stage = flags.keep_stage;

        if config.dry_run {
            let dir = config.out.join("tmp-dry-run");
            t!(fs::create_dir_all(&dir));
            config.out = dir;
        }

        // If --target was specified but --host wasn't specified, don't run any host-only tests.
        config.run_host_only = !(flags.host.is_empty() && !flags.target.is_empty());

        let toml = file.map(|file| {
            let mut f = t!(File::open(&file));
            let mut contents = String::new();
            t!(f.read_to_string(&mut contents));
            match toml::from_str(&contents) {
                Ok(table) => table,
                Err(err) => {
                    println!("failed to parse TOML configuration '{}': {}",
                        file.display(), err);
                    process::exit(2);
                }
            }
        }).unwrap_or_else(|| TomlConfig::default());

        let build = toml.build.clone().unwrap_or(Build::default());
        // set by bootstrap.py
        config.hosts.push(config.build.clone());
        for host in build.host.iter() {
            let host = INTERNER.intern_str(host);
            if !config.hosts.contains(&host) {
                config.hosts.push(host);
            }
        }
        for target in config.hosts.iter().cloned()
            .chain(build.target.iter().map(|s| INTERNER.intern_str(s)))
        {
            if !config.targets.contains(&target) {
                config.targets.push(target);
            }
        }
        config.hosts = if !flags.host.is_empty() {
            flags.host
        } else {
            config.hosts
        };
        config.targets = if !flags.target.is_empty() {
            flags.target
        } else {
            config.targets
        };


        config.nodejs = build.nodejs.map(PathBuf::from);
        config.gdb = build.gdb.map(PathBuf::from);
        config.python = build.python.map(PathBuf::from);
        set(&mut config.low_priority, build.low_priority);
        set(&mut config.compiler_docs, build.compiler_docs);
        set(&mut config.docs, build.docs);
        set(&mut config.submodules, build.submodules);
        set(&mut config.fast_submodules, build.fast_submodules);
        set(&mut config.locked_deps, build.locked_deps);
        set(&mut config.vendor, build.vendor);
        set(&mut config.full_bootstrap, build.full_bootstrap);
        set(&mut config.extended, build.extended);
        config.tools = build.tools;
        set(&mut config.verbose, build.verbose);
        set(&mut config.sanitizers, build.sanitizers);
        set(&mut config.profiler, build.profiler);
        set(&mut config.openssl_static, build.openssl_static);
        set(&mut config.configure_args, build.configure_args);
        set(&mut config.local_rebuild, build.local_rebuild);
        set(&mut config.print_step_timings, build.print_step_timings);
        config.verbose = cmp::max(config.verbose, flags.verbose);

        if let Some(ref install) = toml.install {
            config.prefix = install.prefix.clone().map(PathBuf::from);
            config.sysconfdir = install.sysconfdir.clone().map(PathBuf::from);
            config.datadir = install.datadir.clone().map(PathBuf::from);
            config.docdir = install.docdir.clone().map(PathBuf::from);
            config.bindir = install.bindir.clone().map(PathBuf::from);
            config.libdir = install.libdir.clone().map(PathBuf::from);
            config.mandir = install.mandir.clone().map(PathBuf::from);
        }

        // Store off these values as options because if they're not provided
        // we'll infer default values for them later
        let mut llvm_assertions = None;
        let mut debuginfo_lines = None;
        let mut debuginfo_only_std = None;
        let mut debug = None;
        let mut debug_jemalloc = None;
        let mut debuginfo = None;
        let mut debug_assertions = None;
        let mut optimize = None;
        let mut ignore_git = None;

        if let Some(ref llvm) = toml.llvm {
            match llvm.ccache {
                Some(StringOrBool::String(ref s)) => {
                    config.ccache = Some(s.to_string())
                }
                Some(StringOrBool::Bool(true)) => {
                    config.ccache = Some("ccache".to_string());
                }
                Some(StringOrBool::Bool(false)) | None => {}
            }
            set(&mut config.ninja, llvm.ninja);
            set(&mut config.llvm_enabled, llvm.enabled);
            llvm_assertions = llvm.assertions;
            set(&mut config.llvm_optimize, llvm.optimize);
            set(&mut config.llvm_release_debuginfo, llvm.release_debuginfo);
            set(&mut config.llvm_version_check, llvm.version_check);
            set(&mut config.llvm_static_stdcpp, llvm.static_libstdcpp);
            set(&mut config.llvm_link_shared, llvm.link_shared);
            config.llvm_targets = llvm.targets.clone();
            config.llvm_experimental_targets = llvm.experimental_targets.clone()
                .unwrap_or("WebAssembly".to_string());
            config.llvm_link_jobs = llvm.link_jobs;
        }

        if let Some(ref rust) = toml.rust {
            debug = rust.debug;
            debug_assertions = rust.debug_assertions;
            debuginfo = rust.debuginfo;
            debuginfo_lines = rust.debuginfo_lines;
            debuginfo_only_std = rust.debuginfo_only_std;
            optimize = rust.optimize;
            ignore_git = rust.ignore_git;
            debug_jemalloc = rust.debug_jemalloc;
            set(&mut config.rust_optimize_tests, rust.optimize_tests);
            set(&mut config.rust_debuginfo_tests, rust.debuginfo_tests);
            set(&mut config.codegen_tests, rust.codegen_tests);
            set(&mut config.rust_rpath, rust.rpath);
            set(&mut config.use_jemalloc, rust.use_jemalloc);
            set(&mut config.backtrace, rust.backtrace);
            set(&mut config.channel, rust.channel.clone());
            set(&mut config.rust_dist_src, rust.dist_src);
            set(&mut config.quiet_tests, rust.quiet_tests);
            set(&mut config.test_miri, rust.test_miri);
            set(&mut config.wasm_syscall, rust.wasm_syscall);
            set(&mut config.lld_enabled, rust.lld);
            config.rustc_parallel_queries = rust.experimental_parallel_queries.unwrap_or(false);
            config.rustc_default_linker = rust.default_linker.clone();
            config.musl_root = rust.musl_root.clone().map(PathBuf::from);
            config.save_toolstates = rust.save_toolstates.clone().map(PathBuf::from);

            if let Some(ref backends) = rust.codegen_backends {
                config.rust_codegen_backends = backends.iter()
                    .map(|s| INTERNER.intern_str(s))
                    .collect();
            }

            set(&mut config.rust_codegen_backends_dir, rust.codegen_backends_dir.clone());

            match rust.codegen_units {
                Some(0) => config.rust_codegen_units = Some(num_cpus::get() as u32),
                Some(n) => config.rust_codegen_units = Some(n),
                None => {}
            }
        }

        if let Some(ref t) = toml.target {
            for (triple, cfg) in t {
                let mut target = Target::default();

                if let Some(ref s) = cfg.llvm_config {
                    target.llvm_config = Some(config.src.join(s));
                }
                if let Some(ref s) = cfg.jemalloc {
                    target.jemalloc = Some(config.src.join(s));
                }
                if let Some(ref s) = cfg.android_ndk {
                    target.ndk = Some(config.src.join(s));
                }
                target.cc = cfg.cc.clone().map(PathBuf::from);
                target.cxx = cfg.cxx.clone().map(PathBuf::from);
                target.ar = cfg.ar.clone().map(PathBuf::from);
                target.linker = cfg.linker.clone().map(PathBuf::from);
                target.crt_static = cfg.crt_static.clone();
                target.musl_root = cfg.musl_root.clone().map(PathBuf::from);
                target.qemu_rootfs = cfg.qemu_rootfs.clone().map(PathBuf::from);

                config.target_config.insert(INTERNER.intern_string(triple.clone()), target);
            }
        }

        if let Some(ref t) = toml.dist {
            config.dist_sign_folder = t.sign_folder.clone().map(PathBuf::from);
            config.dist_gpg_password_file = t.gpg_password_file.clone().map(PathBuf::from);
            config.dist_upload_addr = t.upload_addr.clone();
            set(&mut config.rust_dist_src, t.src_tarball);
        }

        // Now that we've reached the end of our configuration, infer the
        // default values for all options that we haven't otherwise stored yet.

        set(&mut config.initial_rustc, build.rustc.map(PathBuf::from));
        set(&mut config.initial_rustc, build.cargo.map(PathBuf::from));

        let default = false;
        config.llvm_assertions = llvm_assertions.unwrap_or(default);

        let default = match &config.channel[..] {
            "stable" | "beta" | "nightly" => true,
            _ => false,
        };
        config.rust_debuginfo_lines = debuginfo_lines.unwrap_or(default);
        config.rust_debuginfo_only_std = debuginfo_only_std.unwrap_or(default);

        let default = debug == Some(true);
        config.debug_jemalloc = debug_jemalloc.unwrap_or(default);
        config.rust_debuginfo = debuginfo.unwrap_or(default);
        config.rust_debug_assertions = debug_assertions.unwrap_or(default);
        config.rust_optimize = optimize.unwrap_or(!default);

        let default = config.channel == "dev";
        config.ignore_git = ignore_git.unwrap_or(default);

        config
    }
