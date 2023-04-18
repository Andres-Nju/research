    pub fn compile() {
        let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap();
        let target_vendor = env::var("CARGO_CFG_TARGET_VENDOR").unwrap();
        let cfg = &mut cc::Build::new();

        cfg.cpp(true);
        cfg.cpp_set_stdlib(None);
        cfg.warnings(false);

        if target_env == "msvc" {
            // Don't pull in extra libraries on MSVC
            cfg.flag("/Zl");
            cfg.flag("/EHsc");
            cfg.define("_CRT_SECURE_NO_WARNINGS", None);
            cfg.define("_LIBUNWIND_DISABLE_VISIBILITY_ANNOTATIONS", None);
        } else {
            cfg.flag("-std=c99");
            cfg.flag("-std=c++11");
            cfg.flag("-nostdinc++");
            cfg.flag("-fno-exceptions");
            cfg.flag("-fno-rtti");
            cfg.flag("-fstrict-aliasing");
            cfg.flag("-funwind-tables");
        }

        let mut unwind_sources = vec![
            "Unwind-EHABI.cpp",
            "Unwind-seh.cpp",
            "Unwind-sjlj.c",
            "UnwindLevel1-gcc-ext.c",
            "UnwindLevel1.c",
            "UnwindRegistersRestore.S",
            "UnwindRegistersSave.S",
            "libunwind.cpp",
        ];

        if target_vendor == "apple" {
            unwind_sources.push("Unwind_AppleExtras.cpp");
        }

        let root = Path::new("../llvm-project/libunwind");
        cfg.include(root.join("include"));
        for src in unwind_sources {
            cfg.file(root.join("src").join(src));
        }

        if target_env == "musl" {
            // use the same C compiler command to compile C++ code so we do not need to setup the
            // C++ compiler env variables on the builders
            cfg.cpp(false);
            // linking for musl is handled in lib.rs
            cfg.cargo_metadata(false);
            println!("cargo:rustc-link-search=native={}", env::var("OUT_DIR").unwrap());
        }

        cfg.compile("unwind");
    }
