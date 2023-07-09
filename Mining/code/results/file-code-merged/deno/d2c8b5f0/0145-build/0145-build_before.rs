// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

use deno_core::error::custom_error;
use deno_core::error::AnyError;
use deno_core::op;
use deno_core::serde::Deserialize;
use deno_core::serde_json::json;
use deno_core::serde_json::Value;
use deno_core::Extension;
use deno_core::JsRuntime;
use deno_core::OpState;
use deno_core::RuntimeOptions;
use regex::Regex;
use std::collections::HashMap;
use std::env;
use std::path::Path;
use std::path::PathBuf;

// TODO(bartlomieju): this module contains a lot of duplicated
// logic with `runtime/build.rs`, factor out to `deno_core`.
fn create_snapshot(
  mut js_runtime: JsRuntime,
  snapshot_path: &Path,
  files: Vec<PathBuf>,
) {
  // TODO(nayeemrmn): https://github.com/rust-lang/cargo/issues/3946 to get the
  // workspace root.
  let display_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
  for file in files {
    println!("cargo:rerun-if-changed={}", file.display());
    let display_path = file.strip_prefix(display_root).unwrap();
    let display_path_str = display_path.display().to_string();
    js_runtime
      .execute_script(
        &("deno:".to_string() + &display_path_str.replace('\\', "/")),
        &std::fs::read_to_string(&file).unwrap(),
      )
      .unwrap();
  }

  let snapshot = js_runtime.snapshot();
  let snapshot_slice: &[u8] = &*snapshot;
  println!("Snapshot size: {}", snapshot_slice.len());

  let compressed_snapshot_with_size = {
    let mut vec = vec![];

    vec.extend_from_slice(
      &u32::try_from(snapshot.len())
        .expect("snapshot larger than 4gb")
        .to_le_bytes(),
    );

    vec.extend_from_slice(
      &zstd::bulk::compress(snapshot_slice, 22)
        .expect("snapshot compression failed"),
    );

    vec
  };

  println!(
    "Snapshot compressed size: {}",
    compressed_snapshot_with_size.len()
  );

  std::fs::write(&snapshot_path, compressed_snapshot_with_size).unwrap();
  println!("Snapshot written to: {} ", snapshot_path.display());
}

#[derive(Debug, Deserialize)]
struct LoadArgs {
  /// The fully qualified specifier that should be loaded.
  specifier: String,
}

fn create_compiler_snapshot(
  snapshot_path: &Path,
  files: Vec<PathBuf>,
  cwd: &Path,
) {
  // libs that are being provided by op crates.
  let mut op_crate_libs = HashMap::new();
  op_crate_libs.insert("deno.cache", deno_cache::get_declaration());
  op_crate_libs.insert("deno.console", deno_console::get_declaration());
  op_crate_libs.insert("deno.url", deno_url::get_declaration());
  op_crate_libs.insert("deno.web", deno_web::get_declaration());
  op_crate_libs.insert("deno.fetch", deno_fetch::get_declaration());
  op_crate_libs.insert("deno.webgpu", deno_webgpu_get_declaration());
  op_crate_libs.insert("deno.websocket", deno_websocket::get_declaration());
  op_crate_libs.insert("deno.webstorage", deno_webstorage::get_declaration());
  op_crate_libs.insert("deno.crypto", deno_crypto::get_declaration());
  op_crate_libs.insert(
    "deno.broadcast_channel",
    deno_broadcast_channel::get_declaration(),
  );
  op_crate_libs.insert("deno.net", deno_net::get_declaration());

  // ensure we invalidate the build properly.
  for (_, path) in op_crate_libs.iter() {
    println!("cargo:rerun-if-changed={}", path.display());
  }

  // libs that should be loaded into the isolate before snapshotting.
  let libs = vec![
    // Deno custom type libraries
    "deno.window",
    "deno.worker",
    "deno.shared_globals",
    "deno.ns",
    "deno.unstable",
    // Deno built-in type libraries
    "es5",
    "es2015.collection",
    "es2015.core",
    "es2015",
    "es2015.generator",
    "es2015.iterable",
    "es2015.promise",
    "es2015.proxy",
    "es2015.reflect",
    "es2015.symbol",
    "es2015.symbol.wellknown",
    "es2016.array.include",
    "es2016",
    "es2017",
    "es2017.intl",
    "es2017.object",
    "es2017.sharedmemory",
    "es2017.string",
    "es2017.typedarrays",
    "es2018.asyncgenerator",
    "es2018.asynciterable",
    "es2018",
    "es2018.intl",
    "es2018.promise",
    "es2018.regexp",
    "es2019.array",
    "es2019",
    "es2019.object",
    "es2019.string",
    "es2019.symbol",
    "es2020.bigint",
    "es2020",
    "es2020.date",
    "es2020.intl",
    "es2020.number",
    "es2020.promise",
    "es2020.sharedmemory",
    "es2020.string",
    "es2020.symbol.wellknown",
    "es2021",
    "es2021.intl",
    "es2021.promise",
    "es2021.string",
    "es2021.weakref",
    "es2022",
    "es2022.array",
    "es2022.error",
    "es2022.intl",
    "es2022.object",
    "es2022.string",
    "esnext",
    "esnext.array",
    "esnext.intl",
  ];

  let path_dts = cwd.join("dts");
  // ensure we invalidate the build properly.
  for name in libs.iter() {
    println!(
      "cargo:rerun-if-changed={}",
      path_dts.join(format!("lib.{}.d.ts", name)).display()
    );
  }

  // create a copy of the vector that includes any op crate libs to be passed
  // to the JavaScript compiler to build into the snapshot
  let mut build_libs = libs.clone();
  for (op_lib, _) in op_crate_libs.iter() {
    build_libs.push(op_lib.to_owned());
  }

  #[op]
  fn op_build_info(state: &mut OpState) -> Value {
    let build_specifier = "asset:///bootstrap.ts";
    let build_libs = state.borrow::<Vec<&str>>();
    json!({
      "buildSpecifier": build_specifier,
      "libs": build_libs,
    })
  }

  #[op]
  fn op_cwd() -> String {
    "cache:///".into()
  }

  #[op]
  fn op_exists() -> bool {
    false
  }

  #[op]
  fn op_script_version(
    _state: &mut OpState,
    _args: Value,
  ) -> Result<Option<String>, AnyError> {
    Ok(Some("1".to_string()))
  }

  #[op]
  // using the same op that is used in `tsc.rs` for loading modules and reading
  // files, but a slightly different implementation at build time.
  fn op_load(state: &mut OpState, args: LoadArgs) -> Result<Value, AnyError> {
    let op_crate_libs = state.borrow::<HashMap<&str, PathBuf>>();
    let path_dts = state.borrow::<PathBuf>();
    let re_asset =
      Regex::new(r"asset:/{3}lib\.(\S+)\.d\.ts").expect("bad regex");
    let build_specifier = "asset:///bootstrap.ts";

    // we need a basic file to send to tsc to warm it up.
    if args.specifier == build_specifier {
      Ok(json!({
        "data": r#"console.log("hello deno!");"#,
        "version": "1",
        // this corresponds to `ts.ScriptKind.TypeScript`
        "scriptKind": 3
      }))
    // specifiers come across as `asset:///lib.{lib_name}.d.ts` and we need to
    // parse out just the name so we can lookup the asset.
    } else if let Some(caps) = re_asset.captures(&args.specifier) {
      if let Some(lib) = caps.get(1).map(|m| m.as_str()) {
        // if it comes from an op crate, we were supplied with the path to the
        // file.
        let path = if let Some(op_crate_lib) = op_crate_libs.get(lib) {
          PathBuf::from(op_crate_lib).canonicalize().unwrap()
        // otherwise we are will generate the path ourself
        } else {
          path_dts.join(format!("lib.{}.d.ts", lib))
        };
        let data = std::fs::read_to_string(path)?;
        Ok(json!({
          "data": data,
          "version": "1",
          // this corresponds to `ts.ScriptKind.TypeScript`
          "scriptKind": 3
        }))
      } else {
        Err(custom_error(
          "InvalidSpecifier",
          format!("An invalid specifier was requested: {}", args.specifier),
        ))
      }
    } else {
      Err(custom_error(
        "InvalidSpecifier",
        format!("An invalid specifier was requested: {}", args.specifier),
      ))
    }
  }
  let js_runtime = JsRuntime::new(RuntimeOptions {
    will_snapshot: true,
    extensions: vec![Extension::builder()
      .ops(vec![
        op_build_info::decl(),
        op_cwd::decl(),
        op_exists::decl(),
        op_load::decl(),
        op_script_version::decl(),
      ])
      .state(move |state| {
        state.put(op_crate_libs.clone());
        state.put(build_libs.clone());
        state.put(path_dts.clone());

        Ok(())
      })
      .build()],
    ..Default::default()
  });

  create_snapshot(js_runtime, snapshot_path, files);
}

fn ts_version() -> String {
  std::fs::read_to_string("tsc/00_typescript.js")
    .unwrap()
    .lines()
    .find(|l| l.contains("ts.version = "))
    .expect(
      "Failed to find the pattern `ts.version = ` in typescript source code",
    )
    .chars()
    .skip_while(|c| !char::is_numeric(*c))
    .take_while(|c| *c != '"')
    .collect::<String>()
}

fn git_commit_hash() -> String {
  if let Ok(output) = std::process::Command::new("git")
    .arg("rev-list")
    .arg("-1")
    .arg("HEAD")
    .output()
  {
    if output.status.success() {
      std::str::from_utf8(&output.stdout[..40])
        .unwrap()
        .to_string()
    } else {
      // When not in git repository
      // (e.g. when the user install by `cargo install deno`)
      "UNKNOWN".to_string()
    }
  } else {
    // When there is no git command for some reason
    "UNKNOWN".to_string()
  }
}

fn main() {
  // Skip building from docs.rs.
  if env::var_os("DOCS_RS").is_some() {
    return;
  }

  // Host snapshots won't work when cross compiling.
  let target = env::var("TARGET").unwrap();
  let host = env::var("HOST").unwrap();
  if target != host {
    panic!("Cross compiling with snapshot is not supported.");
  }

  let symbols_path = std::path::Path::new(
    format!("generated_symbol_exports_list_{}.def", env::consts::OS).as_str(),
  )
  .canonicalize()
  .expect(
    "Missing symbols list! Generate using tools/napi/generate_symbols_lists.js",
  );

  #[cfg(target_os = "windows")]
  println!(
    "cargo:rustc-link-arg-bin=deno=/DEF:{}",
    symbols_path.display()
  );

  #[cfg(target_os = "macos")]
  println!(
    "cargo:rustc-link-arg-bin=deno=-Wl,-exported_symbols_list,{}",
    symbols_path.display()
  );

  #[cfg(target_os = "linux")]
  {
    let ver = glibc_version::get_version().unwrap();
    if ver.major <= 2 && ver.minor < 35 {
      println!("cargo:warning=Compiling with all symbols exported, this will result in a larger binary. Please use glibc 2.35 or later for an optimised build.");
      println!("cargo:rustc-link-arg-bin=deno=-rdynamic");
    } else {
      println!(
        "cargo:rustc-link-arg-bin=deno=-Wl,--export-dynamic-symbol-list={}",
        symbols_path.display()
      );
    }
  }

  // To debug snapshot issues uncomment:
  // op_fetch_asset::trace_serializer();

  if let Ok(c) = env::var("DENO_CANARY") {
    println!("cargo:rustc-env=DENO_CANARY={}", c);
  }
  println!("cargo:rerun-if-env-changed=DENO_CANARY");

  println!("cargo:rustc-env=GIT_COMMIT_HASH={}", git_commit_hash());
  println!("cargo:rerun-if-env-changed=GIT_COMMIT_HASH");

  println!("cargo:rustc-env=TS_VERSION={}", ts_version());
  println!("cargo:rerun-if-env-changed=TS_VERSION");

  println!(
    "cargo:rustc-env=DENO_CONSOLE_LIB_PATH={}",
    deno_console::get_declaration().display()
  );
  println!(
    "cargo:rustc-env=DENO_URL_LIB_PATH={}",
    deno_url::get_declaration().display()
  );
  println!(
    "cargo:rustc-env=DENO_WEB_LIB_PATH={}",
    deno_web::get_declaration().display()
  );
  println!(
    "cargo:rustc-env=DENO_FETCH_LIB_PATH={}",
    deno_fetch::get_declaration().display()
  );
  println!(
    "cargo:rustc-env=DENO_WEBGPU_LIB_PATH={}",
    deno_webgpu_get_declaration().display()
  );
  println!(
    "cargo:rustc-env=DENO_WEBSOCKET_LIB_PATH={}",
    deno_websocket::get_declaration().display()
  );
  println!(
    "cargo:rustc-env=DENO_WEBSTORAGE_LIB_PATH={}",
    deno_webstorage::get_declaration().display()
  );
  println!(
    "cargo:rustc-env=DENO_CACHE_LIB_PATH={}",
    deno_cache::get_declaration().display()
  );
  println!(
    "cargo:rustc-env=DENO_CRYPTO_LIB_PATH={}",
    deno_crypto::get_declaration().display()
  );
  println!(
    "cargo:rustc-env=DENO_BROADCAST_CHANNEL_LIB_PATH={}",
    deno_broadcast_channel::get_declaration().display()
  );
  println!(
    "cargo:rustc-env=DENO_NET_LIB_PATH={}",
    deno_net::get_declaration().display()
  );

  println!("cargo:rustc-env=TARGET={}", env::var("TARGET").unwrap());
  println!("cargo:rustc-env=PROFILE={}", env::var("PROFILE").unwrap());

  let c = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
  let o = PathBuf::from(env::var_os("OUT_DIR").unwrap());

  // Main snapshot
  let compiler_snapshot_path = o.join("COMPILER_SNAPSHOT.bin");

  let js_files = get_js_files("tsc");
  create_compiler_snapshot(&compiler_snapshot_path, js_files, &c);

  #[cfg(target_os = "windows")]
  {
    let mut res = winres::WindowsResource::new();
    res.set_icon("deno.ico");
    res.set_language(winapi::um::winnt::MAKELANGID(
      winapi::um::winnt::LANG_ENGLISH,
      winapi::um::winnt::SUBLANG_ENGLISH_US,
    ));
    res.compile().unwrap();
  }
}

fn deno_webgpu_get_declaration() -> PathBuf {
  let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
  manifest_dir.join("dts").join("lib.deno_webgpu.d.ts")
}

fn get_js_files(d: &str) -> Vec<PathBuf> {
  let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
  let mut js_files = std::fs::read_dir(d)
    .unwrap()
    .map(|dir_entry| {
      let file = dir_entry.unwrap();
      manifest_dir.join(file.path())
    })
    .filter(|path| path.extension().unwrap_or_default() == "js")
    .collect::<Vec<PathBuf>>();
  js_files.sort();
  js_files
}
