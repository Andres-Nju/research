pub fn llvm(build: &Build, target: &str) {
    // If we're using a custom LLVM bail out here, but we can only use a
    // custom LLVM for the build triple.
    if let Some(config) = build.config.target_config.get(target) {
        if let Some(ref s) = config.llvm_config {
            return check_llvm_version(build, s);
        }
    }

    // If the cleaning trigger is newer than our built artifacts (or if the
    // artifacts are missing) then we keep going, otherwise we bail out.
    let dst = build.llvm_out(target);
    let stamp = build.src.join("src/rustllvm/llvm-auto-clean-trigger");
    let llvm_config = dst.join("bin").join(exe("llvm-config", target));
    build.clear_if_dirty(&dst, &stamp);
    if fs::metadata(llvm_config).is_ok() {
        return
    }

    let _ = fs::remove_dir_all(&dst.join("build"));
    t!(fs::create_dir_all(&dst.join("build")));
    let assertions = if build.config.llvm_assertions {"ON"} else {"OFF"};

    // http://llvm.org/docs/CMake.html
    let mut cfg = cmake::Config::new(build.src.join("src/llvm"));
    cfg.target(target)
       .host(&build.config.build)
       .out_dir(&dst)
       .profile(if build.config.llvm_optimize {"Release"} else {"Debug"})
       .define("LLVM_ENABLE_ASSERTIONS", assertions)
       .define("LLVM_TARGETS_TO_BUILD", "X86;ARM;AArch64;Mips;PowerPC")
       .define("LLVM_INCLUDE_EXAMPLES", "OFF")
       .define("LLVM_INCLUDE_TESTS", "OFF")
       .define("LLVM_INCLUDE_DOCS", "OFF")
       .define("LLVM_ENABLE_ZLIB", "OFF")
       .define("WITH_POLLY", "OFF")
       .define("LLVM_ENABLE_TERMINFO", "OFF")
       .define("LLVM_ENABLE_LIBEDIT", "OFF")
       .define("LLVM_PARALLEL_COMPILE_JOBS", build.jobs().to_string());

    if target.starts_with("i686") {
        cfg.define("LLVM_BUILD_32_BITS", "ON");
    }

    // http://llvm.org/docs/HowToCrossCompileLLVM.html
    if target != build.config.build {
        // FIXME: if the llvm root for the build triple is overridden then we
        //        should use llvm-tblgen from there, also should verify that it
        //        actually exists most of the time in normal installs of LLVM.
        let host = build.llvm_out(&build.config.build).join("bin/llvm-tblgen");
        cfg.define("CMAKE_CROSSCOMPILING", "True")
           .define("LLVM_TARGET_ARCH", target.split('-').next().unwrap())
           .define("LLVM_TABLEGEN", &host)
           .define("LLVM_DEFAULT_TARGET_TRIPLE", target);
    }

    // MSVC handles compiler business itself
    if !target.contains("msvc") {
        if build.config.ccache {
           cfg.define("CMAKE_C_COMPILER", "ccache")
              .define("CMAKE_C_COMPILER_ARG1", build.cc(target))
              .define("CMAKE_CXX_COMPILER", "ccache")
              .define("CMAKE_CXX_COMPILER_ARG1", build.cxx(target));
        } else {
           cfg.define("CMAKE_C_COMPILER", build.cc(target))
              .define("CMAKE_CXX_COMPILER", build.cxx(target));
        }
        cfg.build_arg("-j").build_arg(build.jobs().to_string());
    }

    // FIXME: we don't actually need to build all LLVM tools and all LLVM
    //        libraries here, e.g. we just want a few components and a few
    //        tools. Figure out how to filter them down and only build the right
    //        tools and libs on all platforms.
    cfg.build();
}
