fn standalone_follow_redirects() {
  let dir = TempDir::new();
  let exe = if cfg!(windows) {
    dir.path().join("follow_redirects.exe")
  } else {
    dir.path().join("follow_redirects")
  };
  let output = util::deno_cmd()
    .current_dir(util::testdata_path())
    .arg("compile")
    .arg("--unstable")
    .arg("--output")
    .arg(&exe)
    .arg("./standalone_follow_redirects.ts")
    .stdout(std::process::Stdio::piped())
    .spawn()
    .unwrap()
    .wait_with_output()
    .unwrap();
  assert!(output.status.success());
  let output = Command::new(exe)
    .stdout(std::process::Stdio::piped())
    .stderr(std::process::Stdio::piped())
    .spawn()
    .unwrap()
    .wait_with_output()
    .unwrap();
  assert!(output.status.success());
  assert_eq!(output.stdout, b"Hello\n");
}
