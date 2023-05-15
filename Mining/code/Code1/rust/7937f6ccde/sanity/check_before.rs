pub fn check(build: &mut Build) {
    let mut checked = HashSet::new();
    let path = env::var_os("PATH").unwrap_or(OsString::new());
    // On Windows, quotes are invalid characters for filename paths, and if
    // one is present as part of the PATH then that can lead to the system
    // being unable to identify the files properly. See
    // https://github.com/rust-lang/rust/issues/34959 for more details.
    if cfg!(windows) {
        if path.to_string_lossy().contains("\"") {
            panic!("PATH contains invalid character '\"'");
        }
    }
    let have_cmd = |cmd: &OsStr| {
        for path in env::split_paths(&path).map(|p| p.join(cmd)) {
            if fs::metadata(&path).is_ok() ||
               fs::metadata(path.with_extension("exe")).is_ok() {
                return Some(path);
            }
        }
        return None;
    };

    let mut need_cmd = |cmd: &OsStr| {
        if !checked.insert(cmd.to_owned()) {
            return
        }
        if have_cmd(cmd).is_none() {
            panic!("\n\ncouldn't find required command: {:?}\n\n", cmd);
        }
    };

    // If we've got a git directory we're gona need git to update
    // submodules and learn about various other aspects.
    if fs::metadata(build.src.join(".git")).is_ok() {
        need_cmd("git".as_ref());
    }

    // We need cmake, but only if we're actually building LLVM
    for host in build.config.host.iter() {
        if let Some(config) = build.config.target_config.get(host) {
            if config.llvm_config.is_some() {
                continue
            }
        }
        need_cmd("cmake".as_ref());
        if build.config.ninja {
            need_cmd("ninja".as_ref())
        }
        break
    }

    need_cmd("python".as_ref());

    // Look for the nodejs command, needed for emscripten testing
    if let Some(node) = have_cmd("node".as_ref()) {
        build.config.nodejs = Some(node);
    } else if let Some(node) = have_cmd("nodejs".as_ref()) {
        build.config.nodejs = Some(node);
    }

    if let Some(ref s) = build.config.nodejs {
        need_cmd(s.as_ref());
    }

    // We're gonna build some custom C code here and there, host triples
    // also build some C++ shims for LLVM so we need a C++ compiler.
    for target in build.config.target.iter() {
        // On emscripten we don't actually need the C compiler to just
        // build the target artifacts, only for testing. For the sake
        // of easier bot configuration, just skip detection.
        if target.contains("emscripten") {
            continue;
        }

        need_cmd(build.cc(target).as_ref());
        if let Some(ar) = build.ar(target) {
            need_cmd(ar.as_ref());
        }
    }
    for host in build.config.host.iter() {
        need_cmd(build.cxx(host).as_ref());
    }

    // The msvc hosts don't use jemalloc, turn it off globally to
    // avoid packaging the dummy liballoc_jemalloc on that platform.
    for host in build.config.host.iter() {
        if host.contains("msvc") {
            build.config.use_jemalloc = false;
        }
    }

    // Externally configured LLVM requires FileCheck to exist
    let filecheck = build.llvm_filecheck(&build.config.build);
    if !filecheck.starts_with(&build.out) && !filecheck.exists() && build.config.codegen_tests {
        panic!("filecheck executable {:?} does not exist", filecheck);
    }

    for target in build.config.target.iter() {
        // Can't compile for iOS unless we're on OSX
        if target.contains("apple-ios") &&
           !build.config.build.contains("apple-darwin") {
            panic!("the iOS target is only supported on OSX");
        }

        // Make sure musl-root is valid if specified
        if target.contains("musl") && !target.contains("mips") {
            match build.musl_root(target) {
                Some(root) => {
                    if fs::metadata(root.join("lib/libc.a")).is_err() {
                        panic!("couldn't find libc.a in musl dir: {}",
                               root.join("lib").display());
                    }
                    if fs::metadata(root.join("lib/libunwind.a")).is_err() {
                        panic!("couldn't find libunwind.a in musl dir: {}",
                               root.join("lib").display());
                    }
                }
                None => {
                    panic!("when targeting MUSL either the build.musl-root \
                            option or the target.$TARGET.musl-root one must \
                            be specified in config.toml")
                }
            }
        }

        if target.contains("msvc") {
            // There are three builds of cmake on windows: MSVC, MinGW, and
            // Cygwin. The Cygwin build does not have generators for Visual
            // Studio, so detect that here and error.
            let out = output(Command::new("cmake").arg("--help"));
            if !out.contains("Visual Studio") {
                panic!("
cmake does not support Visual Studio generators.

This is likely due to it being an msys/cygwin build of cmake,
rather than the required windows version, built using MinGW
or Visual Studio.

If you are building under msys2 try installing the mingw-w64-x86_64-cmake
package instead of cmake:

$ pacman -R cmake && pacman -S mingw-w64-x86_64-cmake
");
            }
        }

        if target.contains("arm-linux-android") {
            need_cmd("adb".as_ref());
        }
    }

    for host in build.flags.host.iter() {
        if !build.config.host.contains(host) {
            panic!("specified host `{}` is not in the ./configure list", host);
        }
    }
    for target in build.flags.target.iter() {
        if !build.config.target.contains(target) {
            panic!("specified target `{}` is not in the ./configure list",
                   target);
        }
    }

    let run = |cmd: &mut Command| {
        cmd.output().map(|output| {
            String::from_utf8_lossy(&output.stdout)
                   .lines().next().unwrap()
                   .to_string()
        })
    };
    build.gdb_version = run(Command::new("gdb").arg("--version")).ok();
    build.lldb_version = run(Command::new("lldb").arg("--version")).ok();
    if build.lldb_version.is_some() {
        build.lldb_python_dir = run(Command::new("lldb").arg("-P")).ok();
    }
}