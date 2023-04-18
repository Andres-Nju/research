    fn compiler(&self, stage: u32) -> Compiler<'a> {
        Compiler::new(stage, self.target)
    }

    fn target(&self, target: &'a str) -> Step<'a> {
        Step { target: target, src: self.src.clone() }
    }

    // Define ergonomic constructors for each step defined above so they can be
    // easily constructed.
    targets!(constructors);

    /// Mapping of all dependencies for rustbuild.
    ///
    /// This function receives a step, the build that we're building for, and
    /// then returns a list of all the dependencies of that step.
    pub fn deps(&self, build: &'a Build) -> Vec<Step<'a>> {
        match self.src {
            Source::Rustc { stage: 0 } => {
                Vec::new()
            }
            Source::Rustc { stage } => {
                let compiler = Compiler::new(stage - 1, &build.config.build);
                vec![self.librustc(compiler)]
            }
            Source::Librustc { compiler } => {
                vec![self.libtest(compiler), self.llvm(())]
            }
            Source::Libtest { compiler } => {
                vec![self.libstd(compiler)]
            }
            Source::Libstd { compiler } => {
                vec![self.rustc(compiler.stage).target(compiler.host)]
            }
            Source::LibrustcLink { compiler, host } => {
                vec![self.librustc(compiler),
                     self.libtest_link(compiler, host)]
            }
            Source::LibtestLink { compiler, host } => {
                vec![self.libtest(compiler), self.libstd_link(compiler, host)]
            }
            Source::LibstdLink { compiler, host } => {
                vec![self.libstd(compiler),
                     self.target(host).rustc(compiler.stage)]
            }
            Source::Llvm { _dummy } => Vec::new(),
            Source::TestHelpers { _dummy } => Vec::new(),
            Source::DebuggerScripts { stage: _ } => Vec::new(),

            // Note that all doc targets depend on artifacts from the build
            // architecture, not the target (which is where we're generating
            // docs into).
            Source::DocStd { stage } => {
                let compiler = self.target(&build.config.build).compiler(stage);
                vec![self.libstd(compiler)]
            }
            Source::DocTest { stage } => {
                let compiler = self.target(&build.config.build).compiler(stage);
                vec![self.libtest(compiler)]
            }
            Source::DocBook { stage } |
            Source::DocNomicon { stage } => {
                vec![self.target(&build.config.build).tool_rustbook(stage)]
            }
            Source::DocErrorIndex { stage } => {
                vec![self.target(&build.config.build).tool_error_index(stage)]
            }
            Source::DocStandalone { stage } => {
                vec![self.target(&build.config.build).rustc(stage)]
            }
            Source::DocRustc { stage } => {
                vec![self.doc_test(stage)]
            }
            Source::Doc { stage } => {
                let mut deps = vec![
                    self.doc_book(stage), self.doc_nomicon(stage),
                    self.doc_standalone(stage), self.doc_std(stage),
                    self.doc_error_index(stage),
                ];

                if build.config.compiler_docs {
                    deps.push(self.doc_rustc(stage));
                }

                deps
            }
            Source::Check { stage, compiler } => {
                // Check is just a pseudo step which means check all targets,
                // so just depend on checking all targets.
                build.config.target.iter().map(|t| {
                    self.target(t).check_target(stage, compiler)
                }).collect()
            }
            Source::CheckTarget { stage, compiler } => {
                // CheckTarget here means run all possible test suites for this
                // target. Most of the time, however, we can't actually run
                // anything if we're not the build triple as we could be cross
                // compiling.
                //
                // As a result, the base set of targets here is quite stripped
                // down from the standard set of targets. These suites have
                // their own internal logic to run in cross-compiled situations
                // if they'll run at all. For example compiletest knows that
                // when testing Android targets we ship artifacts to the
                // emulator.
                //
                // When in doubt the rule of thumb for adding to this list is
                // "should this test suite run on the android bot?"
                let mut base = vec![
                    self.check_rpass(compiler),
                    self.check_rfail(compiler),
                    self.check_crate_std(compiler),
                    self.check_crate_test(compiler),
                    self.check_debuginfo(compiler),
                ];

                // If we're testing the build triple, then we know we can
                // actually run binaries and such, so we run all possible tests
                // that we know about.
                if self.target == build.config.build {
                    base.extend(vec![
                        // docs-related
                        self.check_docs(compiler),
                        self.check_error_index(compiler),
                        self.check_rustdoc(compiler),

                        // UI-related
                        self.check_cfail(compiler),
                        self.check_pfail(compiler),
                        self.check_ui(compiler),

                        // codegen-related
                        self.check_incremental(compiler),
                        self.check_codegen(compiler),
                        self.check_codegen_units(compiler),

                        // misc compiletest-test suites
                        self.check_rpass_full(compiler),
                        self.check_rfail_full(compiler),
                        self.check_cfail_full(compiler),
                        self.check_pretty_rpass_full(compiler),
                        self.check_pretty_rfail_full(compiler),
                        self.check_rpass_valgrind(compiler),
                        self.check_rmake(compiler),
                        self.check_mir_opt(compiler),

                        // crates
                        self.check_crate_rustc(compiler),

                        // pretty
                        self.check_pretty(compiler),
                        self.check_pretty_rpass(compiler),
                        self.check_pretty_rfail(compiler),
                        self.check_pretty_rpass_valgrind(compiler),

                        // misc
                        self.check_linkcheck(stage),
                        self.check_tidy(stage),

                        // can we make the distributables?
                        self.dist(stage),
                    ]);
                }
                base
            }
            Source::CheckLinkcheck { stage } => {
                vec![self.tool_linkchecker(stage), self.doc(stage)]
            }
            Source::CheckCargoTest { stage } => {
                vec![self.tool_cargotest(stage),
                     self.librustc(self.compiler(stage))]
            }
            Source::CheckTidy { stage } => {
                vec![self.tool_tidy(stage)]
            }
            Source::CheckMirOpt { compiler} |
            Source::CheckPrettyRPass { compiler } |
            Source::CheckPrettyRFail { compiler } |
            Source::CheckRFail { compiler } |
            Source::CheckPFail { compiler } |
            Source::CheckCodegen { compiler } |
            Source::CheckCodegenUnits { compiler } |
            Source::CheckIncremental { compiler } |
            Source::CheckUi { compiler } |
            Source::CheckPretty { compiler } |
            Source::CheckCFail { compiler } |
            Source::CheckRPassValgrind { compiler } |
            Source::CheckRPass { compiler } => {
                let mut base = vec![
                    self.libtest(compiler),
                    self.target(compiler.host).tool_compiletest(compiler.stage),
                    self.test_helpers(()),
                ];
                if self.target.contains("android") {
                    base.push(self.android_copy_libs(compiler));
                }
                base
            }
            Source::CheckDebuginfo { compiler } => {
                vec![
                    self.libtest(compiler),
                    self.target(compiler.host).tool_compiletest(compiler.stage),
                    self.test_helpers(()),
                    self.debugger_scripts(compiler.stage),
                ]
            }
            Source::CheckRustdoc { compiler } |
            Source::CheckRPassFull { compiler } |
            Source::CheckRFailFull { compiler } |
            Source::CheckCFailFull { compiler } |
            Source::CheckPrettyRPassFull { compiler } |
            Source::CheckPrettyRFailFull { compiler } |
            Source::CheckPrettyRPassValgrind { compiler } |
            Source::CheckRMake { compiler } => {
                vec![self.librustc(compiler),
                     self.target(compiler.host).tool_compiletest(compiler.stage)]
            }
            Source::CheckDocs { compiler } => {
                vec![self.libtest(compiler)]
            }
            Source::CheckErrorIndex { compiler } => {
                vec![self.libstd(compiler),
                     self.target(compiler.host).tool_error_index(compiler.stage)]
            }
            Source::CheckCrateStd { compiler } => {
                vec![self.libtest(compiler)]
            }
            Source::CheckCrateTest { compiler } => {
                vec![self.libtest(compiler)]
            }
            Source::CheckCrateRustc { compiler } => {
                vec![self.libtest(compiler)]
            }

            Source::ToolLinkchecker { stage } |
            Source::ToolTidy { stage } => {
                vec![self.libstd(self.compiler(stage))]
            }
            Source::ToolErrorIndex { stage } |
            Source::ToolRustbook { stage } => {
                vec![self.librustc(self.compiler(stage))]
            }
            Source::ToolCargoTest { stage } => {
                vec![self.libstd(self.compiler(stage))]
            }
            Source::ToolCompiletest { stage } => {
                vec![self.libtest(self.compiler(stage))]
            }

            Source::DistDocs { stage } => vec![self.doc(stage)],
            Source::DistMingw { _dummy: _ } => Vec::new(),
            Source::DistRustc { stage } => {
                vec![self.rustc(stage)]
            }
            Source::DistStd { compiler } => {
                // We want to package up as many target libraries as possible
                // for the `rust-std` package, so if this is a host target we
                // depend on librustc and otherwise we just depend on libtest.
                if build.config.host.iter().any(|t| t == self.target) {
                    vec![self.librustc(compiler)]
                } else {
                    vec![self.libtest(compiler)]
                }
            }
            Source::DistSrc { _dummy: _ } => Vec::new(),

            Source::Dist { stage } => {
                let mut base = Vec::new();

                for host in build.config.host.iter() {
                    let host = self.target(host);
                    base.push(host.dist_src(()));
                    base.push(host.dist_rustc(stage));
                    if host.target.contains("windows-gnu") {
                        base.push(host.dist_mingw(()));
                    }

                    let compiler = self.compiler(stage);
                    for target in build.config.target.iter() {
                        let target = self.target(target);
                        if build.config.docs {
                            base.push(target.dist_docs(stage));
                        }
                        base.push(target.dist_std(compiler));
                    }
                }
                base
            }

            Source::Install { stage } => {
                vec![self.dist(stage)]
            }

            Source::AndroidCopyLibs { compiler } => {
                vec![self.libtest(compiler)]
            }
        }
    }
