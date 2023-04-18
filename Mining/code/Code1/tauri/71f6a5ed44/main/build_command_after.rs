fn build_command(matches: &ArgMatches) -> Result<()> {
  let runner = matches.value_of("runner");
  let target = matches.value_of("target");
  let debug = matches.is_present("debug");
  let verbose = matches.is_present("verbose");
  let bundles = matches.values_of_lossy("bundle");
  let config = matches.value_of("config");

  let mut build_runner = build::Build::new();
  if let Some(runner) = runner {
    build_runner = build_runner.runner(runner.to_string());
  }
  if let Some(target) = target {
    build_runner = build_runner.target(target.to_string());
  }
  if debug {
    build_runner = build_runner.debug();
  }
  if verbose {
    build_runner = build_runner.verbose();
  }
  if let Some(bundles) = bundles {
    build_runner = build_runner.bundles(bundles);
  }
  if let Some(config) = config {
    build_runner = build_runner.config(config.to_string());
  }

  build_runner.run()
}
