pub fn exec_cmd(cmd: &str, args: &[&str]) -> Option<CommandOutput> {
    let command = match args.len() {
        0 => String::from(cmd),
        _ => format!("{} {}", cmd, args.join(" ")),
    };
    match command.as_str() {
        "crystal --version" => Some(CommandOutput {
            stdout: String::from("Crystal 0.32.1 (2019-12-18)"),
            stderr: String::default(),
        }),
        "dummy_command" => Some(CommandOutput {
            stdout: String::from("stdout ok!"),
            stderr: String::from("stderr ok!"),
        }),
        "elixir --version" => Some(CommandOutput {
            stdout: String::from(
                "\
Erlang/OTP 22 [erts-10.6.4] [source] [64-bit] [smp:8:8] [ds:8:8:10] [async-threads:1] [hipe]

Elixir 1.10 (compiled with Erlang/OTP 22)",
            ),
            stderr: String::default(),
        }),
        "elm --version" => Some(CommandOutput {
            stdout: String::from("0.19.1"),
            stderr: String::default(),
        }),
        "go version" => Some(CommandOutput {
            stdout: String::from("go version go1.12.1 linux/amd64"),
            stderr: String::default(),
        }),
        "julia --version" => Some(CommandOutput {
            stdout: String::from("julia version 1.4.0"),
            stderr: String::default(),
        }),
        "nim --version" => Some(CommandOutput {
            stdout: String::from(
                "\
Nim Compiler Version 1.2.0 [Linux: amd64]
Compiled at 2020-04-03
Copyright (c) 2006-2020 by Andreas Rumpf
git hash: 7e83adff84be5d0c401a213eccb61e321a3fb1ff
active boot switches: -d:release\n",
            ),
            stderr: String::default(),
        }),
        "node --version" => Some(CommandOutput {
            stdout: String::from("v12.0.0"),
            stderr: String::default(),
        }),
        "ocaml -vnum" => Some(CommandOutput {
            stdout: String::from("4.10.0"),
            stderr: String::default(),
        }),
        "php -r echo PHP_MAJOR_VERSION.'.'.PHP_MINOR_VERSION.'.'.PHP_RELEASE_VERSION;" => {
            Some(CommandOutput {
                stdout: String::from("7.3.8"),
                stderr: String::default(),
            })
        }
        "purs --version" => Some(CommandOutput {
            stdout: String::from("0.13.5"),
            stderr: String::default(),
        }),
        "ruby -v" => Some(CommandOutput {
            stdout: String::from("ruby 2.5.1p57 (2018-03-29 revision 63029) [x86_64-linux-gnu]"),
            stderr: String::default(),
        }),
        "stack --no-install-ghc --lock-file read-only ghc -- --numeric-version" => {
            Some(CommandOutput {
                stdout: String::from("8.6.5"),
                stderr: String::default(),
            })
        }
        "zig version" => Some(CommandOutput {
            stdout: String::from("0.6.0"),
            stderr: String::default(),
        }),
        s if s.starts_with("erl") => Some(CommandOutput {
            stdout: String::from("22.1.3"),
            stderr: String::default(),
        }),
        // If we don't have a mocked command fall back to executing the command
        _ => internal_exec_cmd(&cmd, &args),
    }
}
