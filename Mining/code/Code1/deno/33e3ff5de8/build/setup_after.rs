    pub fn setup() -> Build {
      let gn_mode = if cfg!(target_os = "windows") {
        // On Windows, we need to link with a release build of libdeno, because
        // rust always uses the release CRT.
        // TODO(piscisaureus): make linking with debug libdeno possible.
        String::from("release")
      } else {
        // Cargo sets PROFILE to either "debug" or "release", which conveniently
        // matches the build modes we support.
        env::var("PROFILE").unwrap()
      };

      // cd into workspace root.
      assert!(env::set_current_dir("..").is_ok());

      let root = env::current_dir().unwrap();
      // If not using host default target the output folder will change
      // target/release will become target/$TARGET/release
      // Gn should also be using this output directory as well
      // most things will work with gn using the default
      // output directory but some tests depend on artifacts
      // being in a specific directory relative to the main build output
      let gn_out_path = root.join(format!("target/{}", gn_mode.clone()));
      let gn_out_dir = normalize_path(&gn_out_path);

      // Tell Cargo when to re-run this file. We do this first, so these directives
      // can take effect even if something goes wrong later in the build process.
      println!("cargo:rerun-if-env-changed=DENO_BUILD_PATH");

      // This helps Rust source files locate the snapshot, source map etc.
      println!("cargo:rustc-env=GN_OUT_DIR={}", gn_out_dir);

      // Detect if we're being invoked by the rust language server (RLS).
      // Unfortunately we can't detect whether we're being run by `cargo check`.
      let check_only = env::var_os("CARGO")
        .map(PathBuf::from)
        .as_ref()
        .and_then(|p| p.file_stem())
        .and_then(|f| f.to_str())
        .map(|s| s.starts_with("rls"))
        .unwrap_or(false);

      if check_only {
        // Enable the 'check_only' feature, which enables some workarounds in the
        // rust source code to compile successfully without a bundle and snapshot
        println!("cargo:rustc-cfg=feature=\"check-only\"");
      }

      Build {
        gn_out_dir,
        gn_out_path,
        check_only,
        gn_mode,
        root,
      }
    }

    pub fn run(&self, gn_target: &str) {
