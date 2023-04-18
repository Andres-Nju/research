fn error_if_invalid_cache() {
  let deno_dir = TempDir::new();
  let deno_dir_path = deno_dir.path();
  let tempdir = TempDir::new();
  let tempdir = tempdir.path().join("cov");

  let invalid_cache_path = util::testdata_path().join("coverage/invalid_cache");
  let mod_before_path = util::testdata_path()
    .join(&invalid_cache_path)
    .join("mod_before.ts");
  let mod_after_path = util::testdata_path()
    .join(&invalid_cache_path)
    .join("mod_after.ts");
  let mod_test_path = util::testdata_path()
    .join(&invalid_cache_path)
    .join("mod.test.ts");

  let mod_temp_path = deno_dir_path.join("mod.ts");
  let mod_test_temp_path = deno_dir_path.join("mod.test.ts");

  // Write the inital mod.ts file
  std::fs::copy(mod_before_path, &mod_temp_path).unwrap();
  // And the test file
  std::fs::copy(mod_test_path, mod_test_temp_path).unwrap();

  // Generate coverage
  let status = util::deno_cmd_with_deno_dir(&deno_dir)
    .current_dir(deno_dir_path)
    .arg("test")
    .arg("--quiet")
    .arg(format!("--coverage={}", tempdir.to_str().unwrap()))
    .stdout(std::process::Stdio::piped())
    .stderr(std::process::Stdio::inherit())
    .status()
    .unwrap();

  assert!(status.success());

  // Modify the file between deno test and deno coverage, thus invalidating the cache
  std::fs::copy(mod_after_path, mod_temp_path).unwrap();

  let output = util::deno_cmd_with_deno_dir(&deno_dir)
    .current_dir(deno_dir_path)
    .arg("coverage")
    .arg(format!("{}/", tempdir.to_str().unwrap()))
    .stdout(std::process::Stdio::piped())
    .stderr(std::process::Stdio::piped())
    .output()
    .unwrap();

  assert!(output.stdout.is_empty());

  // Expect error
  let error =
    util::strip_ansi_codes(std::str::from_utf8(&output.stderr).unwrap())
      .to_string();
  assert!(error.contains("error: Missing transpiled source code"));
  assert!(error.contains("Before generating coverage report, run `deno test --coverage` to ensure consistent state."));
}
