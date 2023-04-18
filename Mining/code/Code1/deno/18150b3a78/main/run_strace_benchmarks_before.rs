fn run_strace_benchmarks(
  deno_exe: &PathBuf,
  new_data: &mut BenchResult,
) -> Result<()> {
  use std::io::Read;

  let mut thread_count = HashMap::<String, u64>::new();
  let mut syscall_count = HashMap::<String, u64>::new();

  for (name, args, _) in EXEC_TIME_BENCHMARKS {
    let mut file = tempfile::NamedTempFile::new()?;

    Command::new("strace")
      .args(&[
        "-c",
        "-f",
        "-o",
        file.path().to_str().unwrap(),
        deno_exe.to_str().unwrap(),
      ])
      .args(args.iter())
      .stdout(Stdio::inherit())
      .spawn()?
      .wait()?;

    let mut output = String::new();
    file.as_file_mut().read_to_string(&mut output)?;

    let strace_result = test_util::parse_strace_output(&output);
    let clone = strace_result.get("clone").map(|d| d.calls).unwrap_or(0);
    let total = strace_result.get("total").unwrap().calls;
    thread_count.insert(name.to_string(), clone);
    syscall_count.insert(name.to_string(), total);
  }

  new_data.thread_count = thread_count;
  new_data.syscall_count = syscall_count;

  Ok(())
}
