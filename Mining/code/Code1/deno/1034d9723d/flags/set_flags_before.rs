pub fn set_flags(
  args: Vec<String>,
) -> Result<(DenoFlags, Vec<String>), String> {
  let app_settings: Vec<AppSettings> = vec![
    AppSettings::AllowExternalSubcommands,
    AppSettings::DisableHelpSubcommand,
  ];

  let env_variables_help = "ENVIRONMENT VARIABLES:
    DENO_DIR        Set deno's base directory
    NO_COLOR        Set to disable color";

  let clap_app = App::new("deno")
    .global_settings(&[AppSettings::ColorNever])
    .settings(&app_settings[..])
    .after_help(env_variables_help)
    .arg(
      Arg::with_name("version")
        .short("v")
        .long("version")
        .help("Print the version"),
    ).arg(
      Arg::with_name("allow-read")
        .long("allow-read")
        .help("Allow file system read access"),
    ).arg(
      Arg::with_name("allow-write")
        .long("allow-write")
        .help("Allow file system write access"),
    ).arg(
      Arg::with_name("allow-net")
        .long("allow-net")
        .help("Allow network access"),
    ).arg(
      Arg::with_name("allow-env")
        .long("allow-env")
        .help("Allow environment access"),
    ).arg(
      Arg::with_name("allow-run")
        .long("allow-run")
        .help("Allow running subprocesses"),
    ).arg(
      Arg::with_name("allow-high-precision")
        .long("allow-high-precision")
        .help("Allow high precision time measurement"),
    ).arg(
      Arg::with_name("allow-all")
        .short("A")
        .long("allow-all")
        .help("Allow all permissions"),
    ).arg(
      Arg::with_name("no-prompt")
        .long("no-prompt")
        .help("Do not use prompts"),
    ).arg(
      Arg::with_name("log-debug")
        .short("D")
        .long("log-debug")
        .help("Log debug output"),
    ).arg(
      Arg::with_name("reload")
        .short("r")
        .long("reload")
        .help("Reload source code cache (recompile TypeScript)"),
    ).arg(
      Arg::with_name("v8-options")
        .long("v8-options")
        .help("Print V8 command line options"),
    ).arg(
      Arg::with_name("v8-flags")
        .long("v8-flags")
        .takes_value(true)
        .require_equals(true)
        .help("Set V8 command line options"),
    ).arg(
      Arg::with_name("types")
        .long("types")
        .help("Print runtime TypeScript declarations"),
    ).arg(
      Arg::with_name("prefetch")
        .long("prefetch")
        .help("Prefetch the dependencies"),
    ).subcommand(
      // TODO(bartlomieju): version is not handled properly
      SubCommand::with_name("info")
        .about("Show source file related info")
        .arg(Arg::with_name("file").takes_value(true).required(true)),
    ).subcommand(
      // TODO(bartlomieju): version is not handled properly
      SubCommand::with_name("fmt").about("Format files").arg(
        Arg::with_name("files")
          .takes_value(true)
          .multiple(true)
          .required(true),
      ),
    ).subcommand(
      // this is a fake subcommand - it's used in conjunction with
      // AppSettings:AllowExternalSubcommand to treat it as an
      // entry point script
      SubCommand::with_name("<script>").about("Script to run"),
    );

  let matches = clap_app.get_matches_from(args);

  // TODO(bartomieju): compatibility with old "opts" approach - to be refactored
  let mut rest: Vec<String> = vec![String::from("deno")];

  match matches.subcommand() {
    ("info", Some(info_match)) => {
      // TODO(bartlomieju): it still relies on `is_present("info")` check
      // in `set_recognized_flags`
      let file: &str = info_match.value_of("file").unwrap();
      rest.extend(vec![file.to_string()]);
    }
    ("fmt", Some(fmt_match)) => {
      // TODO(bartlomieju): it still relies on `is_present("fmt")` check
      // in `set_recognized_flags`
      let files: Vec<String> = fmt_match
        .values_of("files")
        .unwrap()
        .map(String::from)
        .collect();
      rest.extend(files);
    }
    (script, Some(script_match)) => {
      rest.extend(vec![script.to_string()]);
      // check if there are any extra arguments
      if script_match.is_present("") {
        let script_args: Vec<String> = script_match
          .values_of("")
          .unwrap()
          .map(String::from)
          .collect();
        rest.extend(script_args);
      }
    }
    _ => {}
  }
  // TODO: end

  if matches.is_present("v8-options") {
    // display v8 help and exit
    v8_set_flags(vec!["deno".to_string(), "--help".to_string()]);
  }

  if matches.is_present("v8-flags") {
    let mut v8_flags: Vec<String> = matches
      .values_of("v8-flags")
      .unwrap()
      .map(String::from)
      .collect();

    v8_flags.insert(1, "deno".to_string());
    v8_set_flags(v8_flags);
  }

  let flags = DenoFlags::from(matches);
  Ok((flags, rest))
}
