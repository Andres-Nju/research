fn captured_output() {
  let output = util::deno_cmd()
    .current_dir(util::testdata_path())
    .arg("test")
    .arg("--allow-run")
    .arg("--allow-read")
    .arg("--unstable")
    .arg("test/captured_output.ts")
    .env("NO_COLOR", "1")
    .stdout(std::process::Stdio::piped())
    .spawn()
    .unwrap()
    .wait_with_output()
    .unwrap();

  let output_start = "------- output -------";
  let output_end = "----- output end -----";
  assert!(output.status.success());
  let output_text = String::from_utf8(output.stdout).unwrap();
  let start = output_text.find(output_start).unwrap() + output_start.len();
  let end = output_text.find(output_end).unwrap();
  let output_text = output_text[start..end].trim();
  let mut lines = output_text.lines().collect::<Vec<_>>();
  // the output is racy on either stdout or stderr being flushed
  // from the runtime into the rust code, so sort it... the main
  // thing here to ensure is that we're capturing the output in
  // this block on stdout
  lines.sort_unstable();
  assert_eq!(lines.join(" "), "0 1 2 3 4 5 6 7 8 9");
}
