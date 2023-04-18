fn main() {
  // To debug snapshot issues uncomment:
  // deno_typescript::trace_serializer();

  println!(
    "cargo:rustc-env=TS_VERSION={}",
    deno_typescript::ts_version()
  );

  let c = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
  let o = PathBuf::from(env::var_os("OUT_DIR").unwrap());

  // Main snapshot
  let root_names = vec![c.join("js/main.ts")];
  let bundle_path = o.join("CLI_SNAPSHOT.js");
  let snapshot_path = o.join("CLI_SNAPSHOT.bin");

  let main_module_name =
    deno_typescript::compile_bundle(&bundle_path, root_names)
      .expect("Bundle compilation failed");
  assert!(bundle_path.exists());

  let runtime_isolate = &mut Isolate::new(StartupData::None, true);

  deno_typescript::mksnapshot_bundle(
    runtime_isolate,
    &snapshot_path,
    &bundle_path,
    &main_module_name,
  )
  .expect("Failed to create snapshot");

  // Compiler snapshot
  let root_names = vec![c.join("js/compiler.ts")];
  let bundle_path = o.join("COMPILER_SNAPSHOT.js");
  let snapshot_path = o.join("COMPILER_SNAPSHOT.bin");
  let mut custom_libs: HashMap<String, PathBuf> = HashMap::new();
  custom_libs.insert(
    "lib.deno.window.d.ts".to_string(),
    c.join("js/lib.deno.window.d.ts"),
  );
  custom_libs.insert(
    "lib.deno.worker.d.ts".to_string(),
    c.join("js/lib.deno.worker.d.ts"),
  );
  custom_libs.insert(
    "lib.deno.shared_globals.d.ts".to_string(),
    c.join("js/lib.deno.shared_globals.d.ts"),
  );
  custom_libs.insert(
    "lib.deno.ns.d.ts".to_string(),
    c.join("js/lib.deno.ns.d.ts"),
  );

  let main_module_name =
    deno_typescript::compile_bundle(&bundle_path, root_names)
      .expect("Bundle compilation failed");
  assert!(bundle_path.exists());

  let runtime_isolate = &mut Isolate::new(StartupData::None, true);
  runtime_isolate.register_op("fetch_asset", op_fetch_asset(custom_libs));

  deno_typescript::mksnapshot_bundle_ts(
    runtime_isolate,
    &snapshot_path,
    &bundle_path,
    &main_module_name,
  )
  .expect("Failed to create snapshot");
}
