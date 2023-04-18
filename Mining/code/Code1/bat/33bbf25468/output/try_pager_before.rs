    fn try_pager(quit_if_one_screen: bool) -> Self {
        let mut args = vec!["--RAW-CONTROL-CHARS", "--no-init"];
        if quit_if_one_screen {
            args.push("--quit-if-one-screen");
        }
        Command::new("less")
            .args(&args)
            .stdin(Stdio::piped())
            .spawn()
            .map(OutputType::Pager)
            .unwrap_or_else(|_| OutputType::stdout())
    }
