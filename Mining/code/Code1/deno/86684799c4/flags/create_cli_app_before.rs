pub fn create_cli_app<'a, 'b>() -> App<'a, 'b> {
  App::new("deno")
    .bin_name("deno")
    .global_settings(&[AppSettings::ColorNever])
    .settings(&[AppSettings::DisableVersion])
    .after_help(ENV_VARIABLES_HELP)
    .long_about("A secure runtime for JavaScript and TypeScript built with V8, Rust, and Tokio.

Docs: https://deno.land/manual.html
Modules: https://github.com/denoland/deno_std
Bugs: https://github.com/denoland/deno/issues

To run the REPL:

  deno

To execute a sandboxed script:

  deno run https://deno.land/welcome.ts

To evaluate code from the command line:

  deno eval \"console.log(30933 + 404)\"

To get help on the another subcommands (run in this case):

  deno help run")
    .arg(
      Arg::with_name("log-debug")
        .short("D")
        .long("log-debug")
        .help("Log debug output")
        .global(true),
    ).arg(
      Arg::with_name("reload")
        .short("r")
        .long("reload")
        .help("Reload source code cache (recompile TypeScript)")
        .global(true),
    ).arg(
      Arg::with_name("config")
        .short("c")
        .long("config")
        .value_name("FILE")
        .help("Load compiler configuration file")
        .takes_value(true)
        .global(true),
    ).arg(
      Arg::with_name("v8-options")
        .long("v8-options")
        .help("Print V8 command line options")
        .global(true),
    ).arg(
      Arg::with_name("v8-flags")
        .long("v8-flags")
        .takes_value(true)
        .use_delimiter(true)
        .require_equals(true)
        .help("Set V8 command line options")
        .global(true),
    ).subcommand(
      SubCommand::with_name("version")
        .setting(AppSettings::DisableVersion)
        .about("Print the version")
        .long_about("Print current version of Deno.

Includes versions of Deno, V8 JavaScript Engine, and the TypeScript
compiler.",
        ),
    ).subcommand(
      SubCommand::with_name("fetch")
        .setting(AppSettings::DisableVersion)
        .about("Fetch the dependencies")
        .long_about(
          "Fetch and compile remote dependencies recursively.

Downloads all statically imported scripts and save them in local
cache, without running the code. No future import network requests
would be made unless --reload is specified.

  # Downloads all dependencies
  deno fetch https://deno.land/std/http/file_server.ts

  # Once cached, static imports no longer send network requests
  deno run -A https://deno.land/std/http/file_server.ts",
        ).arg(Arg::with_name("file").takes_value(true).required(true)),
    ).subcommand(
      SubCommand::with_name("types")
        .setting(AppSettings::DisableVersion)
        .about("Print runtime TypeScript declarations")
        .long_about("Print runtime TypeScript declarations.

  deno types > lib.deno_runtime.d.ts

The declaration file could be saved and used for typing information.",
        ),
    ).subcommand(
      SubCommand::with_name("info")
        .setting(AppSettings::DisableVersion)
        .about("Show source file related info")
        .long_about("Show source file related info.

  deno info https://deno.land/std@v0.6/http/file_server.ts

The following information is shown:

  local:    Local path of the file.
  type:     JavaScript, TypeScript, or JSON.
  compiled: TypeScript only. shown local path of compiled source code.
  map:      TypeScript only. shown local path of source map.
  deps:     Dependency tree of the source file.",
        ).arg(Arg::with_name("file").takes_value(true).required(true)),
    ).subcommand(
      SubCommand::with_name("eval")
        .setting(AppSettings::DisableVersion)
        .about("Eval script")
        .long_about(
          "Evaluate provided script.

This command has implicit access to all permissions (equivalent to deno run --allow-all)

  deno eval \"console.log('hello world')\"",
        ).arg(Arg::with_name("code").takes_value(true).required(true)),
    ).subcommand(
      SubCommand::with_name("fmt")
        .setting(AppSettings::DisableVersion)
        .about("Format files")
        .long_about(
"Auto-format JavaScript/TypeScript source code using Prettier

Automatically downloads Prettier dependencies on first run.

  deno fmt --write myfile1.ts myfile2.ts",
        ).arg(
          Arg::with_name("files")
            .takes_value(true)
            .multiple(true)
            .required(true),
        ),
    ).subcommand(
      SubCommand::with_name("run")
        .settings(&[
          AppSettings::AllowExternalSubcommands,
          AppSettings::DisableHelpSubcommand,
          AppSettings::DisableVersion,
          AppSettings::SubcommandRequired,
        ]).about("Run a program given a filename or url to the source code")
        .long_about(
          "Run a program given a filename or url to the source code.

By default all programs are run in sandbox without access to disk, network or
ability to spawn subprocesses.

  deno run https://deno.land/welcome.ts

  # run program with permission to read from disk and listen to network
  deno run --allow-net --allow-read https://deno.land/std/http/file_server.ts

  # run program with permission to read whitelist files from disk and listen to nework
  deno run --allow-net --allow-read=$(pwd) https://deno.land/std/http/file_server.ts 

  # run program with all permissions
  deno run -A https://deno.land/std/http/file_server.ts",
        ).arg(
          Arg::with_name("allow-read")
        .long("allow-read")
        .min_values(0)
        .takes_value(true)
        .use_delimiter(true)
        .require_equals(true)
        .help("Allow file system read access"),
    ).arg(
      Arg::with_name("allow-write")
        .long("allow-write")
        .min_values(0)
        .takes_value(true)
        .use_delimiter(true)
        .require_equals(true)
        .help("Allow file system write access"),
    ).arg(
      Arg::with_name("allow-net")
        .long("allow-net")
        .min_values(0)
        .takes_value(true)
        .use_delimiter(true)
        .require_equals(true)
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
          Arg::with_name("allow-hrtime")
            .long("allow-hrtime")
            .help("Allow high resolution time measurement"),
        ).arg(
          Arg::with_name("allow-all")
            .short("A")
            .long("allow-all")
            .help("Allow all permissions"),
        ).arg(
          Arg::with_name("no-prompt")
            .long("no-prompt")
            .help("Do not use prompts"),
        ).subcommand(
          // this is a fake subcommand - it's used in conjunction with
          // AppSettings:AllowExternalSubcommand to treat it as an
          // entry point script
          SubCommand::with_name("<script>").about("Script to run"),
        ),
    ).subcommand(
    SubCommand::with_name("xeval")
        .setting(AppSettings::DisableVersion)
        .about("Eval a script on text segments from stdin")
        .long_about(
          "Eval a script on lines from stdin

Read from standard input and eval code on each whitespace-delimited
string chunks.

-I/--replvar optionally sets variable name for input to be used in eval.
Otherwise '$' will be used as default variable name.

This command has implicit access to all permissions (equivalent to deno run --allow-all)

Print all the usernames in /etc/passwd:

  cat /etc/passwd | deno xeval \"a = $.split(':'); if (a) console.log(a[0])\"

A complicated way to print the current git branch:

  git branch | deno xeval -I 'line' \"if (line.startsWith('*')) console.log(line.slice(2))\"

Demonstrates breaking the input up by space delimiter instead of by lines:

  cat LICENSE | deno xeval -d \" \" \"if ($ === 'MIT') console.log('MIT licensed')\"",
        ).arg(
          Arg::with_name("replvar")
            .long("replvar")
            .short("I")
            .help("Set variable name to be used in eval, defaults to $")
            .takes_value(true),
        ).arg(
          Arg::with_name("delim")
            .long("delim")
            .short("d")
            .help("Set delimiter, defaults to newline")
            .takes_value(true),
        ).arg(Arg::with_name("code").takes_value(true).required(true)),
    )
}
