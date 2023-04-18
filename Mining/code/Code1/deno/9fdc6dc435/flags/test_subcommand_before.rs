fn test_subcommand<'a, 'b>() -> App<'a, 'b> {
  run_test_args(SubCommand::with_name("test"))
    .arg(
      Arg::with_name("failfast")
        .long("failfast")
        .help("Stop on first error")
        .takes_value(false),
    )
    .arg(
      Arg::with_name("allow_none")
        .long("allow-none")
        .help("Don't return error code if no test files are found")
        .takes_value(false),
    )
    .arg(
      Arg::with_name("filter")
        .long("filter")
        .takes_value(true)
        .help("A pattern to filter the tests to run by"),
    )
    .arg(
      Arg::with_name("files")
        .help("List of file names to run")
        .takes_value(true)
        .multiple(true),
    )
    .about("Run tests")
    .long_about(
      "Run tests using Deno's built-in test runner.

Evaluate the given modules, run all tests declared with 'Deno.test()' and
report results to standard output:
  deno test src/fetch_test.ts src/signal_test.ts

Directory arguments are expanded to all contained files matching the glob
{*_,}test.{js,ts,jsx,tsx}:
  deno test src/",
    )
}
