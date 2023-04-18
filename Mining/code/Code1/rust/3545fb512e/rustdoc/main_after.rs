fn main() {
    let args = env::args_os().skip(1).collect::<Vec<_>>();
    let rustdoc = env::var_os("RUSTDOC_REAL").expect("RUSTDOC_REAL was not set");
    let libdir = env::var_os("RUSTC_LIBDIR").expect("RUSTC_LIBDIR was not set");
    let stage = env::var("RUSTC_STAGE").expect("RUSTC_STAGE was not set");
    let sysroot = env::var_os("RUSTC_SYSROOT").expect("RUSTC_SYSROOT was not set");

    let mut dylib_path = bootstrap::util::dylib_path();
    dylib_path.insert(0, PathBuf::from(libdir));

    let mut cmd = Command::new(rustdoc);
    cmd.args(&args)
        .arg("--cfg")
        .arg(format!("stage{}", stage))
        .arg("--cfg")
        .arg("dox")
        .arg("--sysroot")
        .arg(sysroot)
        .env(bootstrap::util::dylib_path_var(),
             env::join_paths(&dylib_path).unwrap());

    // Pass the `rustbuild` feature flag to crates which rustbuild is
    // building. See the comment in bootstrap/lib.rs where this env var is
    // set for more details.
    if env::var_os("RUSTBUILD_UNSTABLE").is_some() {
        cmd.arg("--cfg").arg("rustbuild");
    }

    std::process::exit(match cmd.status() {
        Ok(s) => s.code().unwrap_or(1),
        Err(e) => panic!("\n\nfailed to run {:?}: {}\n\n", cmd, e),
    })
}
