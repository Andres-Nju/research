fn install_client(ClientOpt::VsCode: ClientOpt) -> Result<()> {
    let npm_version = Cmd {
        unix: r"npm --version",
        windows: r"cmd.exe /c npm.cmd --version",
        work_dir: "./editors/code",
    }
    .run();

    if npm_version.is_err() {
        eprintln!("\nERROR: `npm --version` failed, `npm` is required to build the VS Code plugin")
    }

    Cmd { unix: r"npm ci", windows: r"cmd.exe /c npm.cmd ci", work_dir: "./editors/code" }.run()?;
    Cmd {
        unix: r"npm run package",
        windows: r"cmd.exe /c npm.cmd run package",
        work_dir: "./editors/code",
    }
    .run()?;

    let code_binary = ["code", "code-insiders", "codium"].iter().find(|bin| {
        Cmd {
            unix: &format!("{} --version", bin),
            windows: &format!("cmd.exe /c {}.cmd --version", bin),
            work_dir: "./editors/code",
        }
        .run()
        .is_ok()
    });

    let code_binary = match code_binary {
        Some(it) => it,
        None => Err("Can't execute `code --version`. Perhaps it is not in $PATH?")?,
    };

    Cmd {
        unix: &format!(r"{} --install-extension ./ra-lsp-0.0.1.vsix --force", code_binary),
        windows: &format!(
            r"cmd.exe /c {}.cmd --install-extension ./ra-lsp-0.0.1.vsix --force",
            code_binary
        ),
        work_dir: "./editors/code",
    }
    .run()?;

    let output = Cmd {
        unix: &format!(r"{} --list-extensions", code_binary),
        windows: &format!(r"cmd.exe /c {}.cmd --list-extensions", code_binary),
        work_dir: ".",
    }
    .run_with_output()?;

    if !str::from_utf8(&output.stdout)?.contains("ra-lsp") {
        Err("Could not install the Visual Studio Code extension. \
             Please make sure you have at least NodeJS 10.x installed and try again.")?;
    }

    Ok(())
}
