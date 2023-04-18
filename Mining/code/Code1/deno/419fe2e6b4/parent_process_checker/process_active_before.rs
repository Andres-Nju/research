  fn process_active() {
    // launch a long running process
    let mut child = Command::new(deno_exe_path()).arg("lsp").spawn().unwrap();

    let pid = child.id();
    assert_eq!(is_process_active(pid), true);
    child.kill().unwrap();
    child.wait().unwrap();
    assert_eq!(is_process_active(pid), false);
  }
