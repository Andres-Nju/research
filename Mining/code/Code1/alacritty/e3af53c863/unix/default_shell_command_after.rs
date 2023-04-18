fn default_shell_command(pw: &Passwd<'_>) -> Command {
    Command::new(default_shell(pw))
}

#[cfg(target_os = "macos")]
fn default_shell_command(pw: &Passwd<'_>) -> Command {
    let shell = default_shell(pw);
    let shell_name = shell.rsplit('/').next().unwrap();

    // On macOS, use the `login` command so the shell will appear as a tty session.
    let mut login_command = Command::new("/usr/bin/login");

    // Exec the shell with argv[0] prepended by '-' so it becomes a login shell.
    // `login` normally does this itself, but `-l` disables this.
    let exec = format!("exec -a -{} {}", shell_name, shell);

    // -f: Bypasses authentication for the already-logged-in user.
    // -l: Skips changing directory to $HOME and prepending '-' to argv[0].
    // -p: Preserves the environment.
    //
    // XXX: we use zsh here over sh due to `exec -a`.
    login_command.args(["-flp", pw.name, "/bin/zsh", "-c", &exec]);
    login_command
}
