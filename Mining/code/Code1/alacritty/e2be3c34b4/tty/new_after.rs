pub fn new<T: ToWinsize>(config: &Config, options: &Options, size: T) -> Pty {
    let win = size.to_winsize();
    let mut buf = [0; 1024];
    let pw = get_pw_entry(&mut buf);

    let (master, slave) = openpty(win.ws_row as _, win.ws_col as _);

    let default_shell = &Shell::new(pw.shell);
    let shell = config.shell()
        .unwrap_or(&default_shell);

    let initial_command = options.command().unwrap_or(&shell);

    let mut builder = Command::new(initial_command.program());
    for arg in initial_command.args() {
        builder.arg(arg);
    }

    // Setup child stdin/stdout/stderr as slave fd of pty
    // Ownership of fd is transferred to the Stdio structs and will be closed by them at the end of
    // this scope. (It is not an issue that the fd is closed three times since File::drop ignores
    // error on libc::close.)
    builder.stdin(unsafe { Stdio::from_raw_fd(slave) });
    builder.stderr(unsafe { Stdio::from_raw_fd(slave) });
    builder.stdout(unsafe { Stdio::from_raw_fd(slave) });

    // Setup environment
    builder.env("LOGNAME", pw.name);
    builder.env("USER", pw.name);
    builder.env("SHELL", shell.program());
    builder.env("HOME", pw.dir);
    builder.env("TERM", "xterm-256color"); // default term until we can supply our own
    for (key, value) in config.env().iter() {
        builder.env(key, value);
    }

    builder.before_exec(move || {
        // Create a new process group
        unsafe {
            let err = libc::setsid();
            if err == -1 {
                die!("Failed to set session id: {}", errno());
            }
        }

        set_controlling_terminal(slave);

        // No longer need slave/master fds
        unsafe {
            libc::close(slave);
            libc::close(master);
        }

        unsafe {
            libc::signal(libc::SIGCHLD, libc::SIG_DFL);
            libc::signal(libc::SIGHUP, libc::SIG_DFL);
            libc::signal(libc::SIGINT, libc::SIG_DFL);
            libc::signal(libc::SIGQUIT, libc::SIG_DFL);
            libc::signal(libc::SIGTERM, libc::SIG_DFL);
            libc::signal(libc::SIGALRM, libc::SIG_DFL);
        }
        Ok(())
    });

    // Handle set working directory option
    if let Some(ref dir) = options.working_dir {
        builder.current_dir(dir.as_path());
    }

    match builder.spawn() {
        Ok(child) => {
            unsafe {
                // Set PID for SIGCHLD handler
                PID = child.id() as _;

                // Handle SIGCHLD
                libc::signal(SIGCHLD, sigchld as _);
            }
            unsafe {
                // Maybe this should be done outside of this function so nonblocking
                // isn't forced upon consumers. Although maybe it should be?
                set_nonblocking(master);
            }

            let pty = Pty { fd: master };
            pty.resize(size);
            pty
        },
        Err(err) => {
            die!("Command::spawn() failed: {}", err);
        }
    }
}
