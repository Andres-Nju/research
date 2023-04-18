  fn process_active() {
    // launch a long running process
    let mut child = Command::new(deno_exe_path()).arg("lsp").spawn().unwrap();

    let pid = child.id();
    assert!(is_process_active(pid));
    child.kill().unwrap();
    child.wait().unwrap();
    assert!(!is_process_active(pid));
  }
