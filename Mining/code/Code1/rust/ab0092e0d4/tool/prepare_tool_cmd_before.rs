    fn prepare_tool_cmd(&self, compiler: Compiler, tool: Tool, cmd: &mut Command) {
        let host = &compiler.host;
        let mut lib_paths: Vec<PathBuf> = vec![
            if compiler.stage == 0 {
                self.build.rustc_snapshot_libdir()
            } else {
                PathBuf::from(&self.sysroot_libdir(compiler, compiler.host))
            },
            self.cargo_out(compiler, tool.get_mode(), *host).join("deps"),
        ];

        // On MSVC a tool may invoke a C compiler (e.g. compiletest in run-make
        // mode) and that C compiler may need some extra PATH modification. Do
        // so here.
        if compiler.host.contains("msvc") {
            let curpaths = env::var_os("PATH").unwrap_or_default();
            let curpaths = env::split_paths(&curpaths).collect::<Vec<_>>();
            for &(ref k, ref v) in self.cc[&compiler.host].env() {
                if k != "PATH" {
                    continue
                }
                for path in env::split_paths(v) {
                    if !curpaths.contains(&path) {
                        lib_paths.push(path);
                    }
                }
            }
        }

        // Add the llvm/bin directory to PATH since it contains lots of
        // useful, platform-independent tools
        if tool.uses_llvm_tools() {
            if let Some(llvm_bin_path) = self.llvm_bin_path() {
                if host.contains("windows") {
                    // On Windows, PATH and the dynamic library path are the same,
                    // so we just add the LLVM bin path to lib_path
                    lib_paths.push(llvm_bin_path);
                } else {
                    let old_path = env::var_os("PATH").unwrap_or_default();
                    let new_path = env::join_paths(iter::once(llvm_bin_path)
                            .chain(env::split_paths(&old_path)))
                        .expect("Could not add LLVM bin path to PATH");
                    cmd.env("PATH", new_path);
                }
            }
        }

        add_lib_path(lib_paths, cmd);
    }
