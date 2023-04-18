fn get_shell<'a, 'b>(shell_args: &'b [&'a str]) -> (std::borrow::Cow<'a, str>, &'b [&'a str]) {
    if !shell_args.is_empty() {
        (shell_args[0].into(), &shell_args[1..])
    } else if let Ok(env_shell) = std::env::var("STARSHIP_SHELL") {
        (env_shell.into(), &[] as &[&str])
    } else {
        ("sh".into(), &[] as &[&str])
    }
}

/// Attempt to run the given command in a shell by passing it as `stdin` to `get_shell()`
#[cfg(not(windows))]
fn shell_command(cmd: &str, shell_args: &[&str]) -> Option<Output> {
    let (shell, shell_args) = get_shell(shell_args);
    let mut command = Command::new(shell.as_ref());

    command
        .args(shell_args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    handle_powershell(&mut command, &shell, shell_args);

    let mut child = match command.spawn() {
        Ok(command) => command,
        Err(err) => {
            log::trace!("Error executing command: {:?}", err);
            log::debug!(
                "Could not launch command with given shell or STARSHIP_SHELL env variable, retrying with /usr/bin/env sh"
            );

            Command::new("/usr/bin/env")
                .arg("sh")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .ok()?
        }
    };

    child.stdin.as_mut()?.write_all(cmd.as_bytes()).ok()?;
    child.wait_with_output().ok()
}

/// Attempt to run the given command in a shell by passing it as `stdin` to `get_shell()`,
/// or by invoking cmd.exe /C.
#[cfg(windows)]
fn shell_command(cmd: &str, shell_args: &[&str]) -> Option<Output> {
    let (shell, shell_args) = if !shell_args.is_empty() {
        (
            Some(std::borrow::Cow::Borrowed(shell_args[0])),
            &shell_args[1..],
        )
    } else if let Ok(env_shell) = std::env::var("STARSHIP_SHELL") {
        (Some(std::borrow::Cow::Owned(env_shell)), &[] as &[&str])
    } else {
        (None, &[] as &[&str])
    };

    if let Some(forced_shell) = shell {
        let mut command = Command::new(forced_shell.as_ref());

        command
            .args(shell_args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        handle_powershell(&mut command, &forced_shell, shell_args);

        if let Ok(mut child) = command.spawn() {
            child.stdin.as_mut()?.write_all(cmd.as_bytes()).ok()?;

            return child.wait_with_output().ok();
        }

        log::debug!(
            "Could not launch command with given shell or STARSHIP_SHELL env variable, retrying with cmd.exe /C"
        );
    }

    let command = Command::new("cmd.exe")
        .arg("/C")
        .arg(cmd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();

    command.ok()?.wait_with_output().ok()
}

/// Execute the given command capturing all output, and return whether it return 0
fn exec_when(cmd: &str, shell_args: &[&str]) -> bool {
    log::trace!("Running '{}'", cmd);
