fn dirty_both_lib_and_test() {
    // This tests that all artifacts that depend on the results of a build
    // script will get rebuilt when the build script reruns, even for separate
    // commands. It does the following:
    //
    // 1. Project "foo" has a build script which will compile a small
    //    staticlib to link against. Normally this would use the `cc` crate,
    //    but here we just use rustc to avoid the `cc` dependency.
    // 2. Build the library.
    // 3. Build the unit test. The staticlib intentionally has a bad value.
    // 4. Rewrite the staticlib with the correct value.
    // 5. Build the library again.
    // 6. Build the unit test. This should recompile.

    let slib = |n| {
        format!(
            r#"
            #[no_mangle]
            pub extern "C" fn doit() -> i32 {{
                return {};
            }}
        "#,
            n
        )
    };

    let p = project()
        .file(
            "src/lib.rs",
            r#"
            extern "C" {
                fn doit() -> i32;
            }

            #[test]
            fn t1() {
                assert_eq!(unsafe { doit() }, 1, "doit assert failure");
            }
        "#,
        )
        .file(
            "build.rs",
            r#"
            use std::env;
            use std::path::PathBuf;
            use std::process::Command;

            fn main() {
                let rustc = env::var_os("RUSTC").unwrap();
                let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
                assert!(
                    Command::new(rustc)
                        .args(&[
                            "--crate-type=staticlib",
                            "--out-dir",
                            out_dir.to_str().unwrap(),
                            "slib.rs"
                        ])
                        .status()
                        .unwrap()
                        .success(),
                    "slib build failed"
                );
                println!("cargo:rustc-link-lib=slib");
                println!("cargo:rustc-link-search={}", out_dir.display());
            }
        "#,
        )
        .file("slib.rs", &slib(2))
        .build();

    p.cargo("build").run();

    // 2 != 1
    p.cargo("test --lib")
        .with_status(101)
        .with_stdout_contains("[..]doit assert failure[..]")
        .run();

    if is_coarse_mtime() {
        // #5918
        sleep_ms(1000);
    }
    // Fix the mistake.
    p.change_file("slib.rs", &slib(1));

    p.cargo("build").run();
    // This should recompile with the new static lib, and the test should pass.
    p.cargo("test --lib").run();
}
