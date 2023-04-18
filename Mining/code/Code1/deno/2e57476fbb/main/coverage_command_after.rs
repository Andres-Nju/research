async fn coverage_command(
  flags: Flags,
  files: Vec<PathBuf>,
  ignore: Vec<PathBuf>,
  include: Vec<String>,
  exclude: Vec<String>,
  lcov: bool,
) -> Result<(), AnyError> {
  if files.is_empty() {
    return Err(generic_error("No matching coverage profiles found"));
  }

  tools::coverage::cover_files(
    flags.clone(),
    files,
    ignore,
    include,
    exclude,
    lcov,
  )
  .await
}
