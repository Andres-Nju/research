    unsafe fn do_exec(&mut self, stdio: ChildPipes) -> io::Error {
        macro_rules! t {
            ($e:expr) => (match $e {
                Ok(e) => e,
                Err(e) => return e,
            })
        }

        if let Some(fd) = stdio.stderr.fd() {
            t!(cvt(syscall::dup2(fd, 2, &[])));
            let mut flags = t!(cvt(syscall::fcntl(2, syscall::F_GETFD, 0)));
            flags &= ! syscall::O_CLOEXEC;
            t!(cvt(syscall::fcntl(2, syscall::F_SETFD, flags)));
        }
        if let Some(fd) = stdio.stdout.fd() {
            t!(cvt(syscall::dup2(fd, 1, &[])));
            let mut flags = t!(cvt(syscall::fcntl(1, syscall::F_GETFD, 0)));
            flags &= ! syscall::O_CLOEXEC;
            t!(cvt(syscall::fcntl(1, syscall::F_SETFD, flags)));
        }
        if let Some(fd) = stdio.stdin.fd() {
            t!(cvt(syscall::dup2(fd, 0, &[])));
            let mut flags = t!(cvt(syscall::fcntl(0, syscall::F_GETFD, 0)));
            flags &= ! syscall::O_CLOEXEC;
            t!(cvt(syscall::fcntl(0, syscall::F_SETFD, flags)));
        }

        if let Some(g) = self.gid {
            t!(cvt(syscall::setregid(g as usize, g as usize)));
        }
        if let Some(u) = self.uid {
            t!(cvt(syscall::setreuid(u as usize, u as usize)));
        }
        if let Some(ref cwd) = self.cwd {
            t!(cvt(syscall::chdir(cwd)));
        }

        for callback in self.closures.iter_mut() {
            t!(callback());
        }

        let mut args: Vec<[usize; 2]> = Vec::new();
        args.push([self.program.as_ptr() as usize, self.program.len()]);
        for arg in self.args.iter() {
            args.push([arg.as_ptr() as usize, arg.len()]);
        }

        for (key, val) in self.env.iter() {
            env::set_var(key, val);
        }

        let program = if self.program.contains(':') || self.program.contains('/') {
            Some(PathBuf::from(&self.program))
        } else if let Ok(path_env) = ::env::var("PATH") {
            let mut program = None;
            for mut path in split_paths(&path_env) {
                path.push(&self.program);
                if path.exists() {
                    program = Some(path);
                    break;
                }
            }
            program
        } else {
            None
        };

        if let Some(program) = program {
            if let Err(err) = syscall::execve(program.as_os_str().as_bytes(), &args) {
                io::Error::from_raw_os_error(err.errno as i32)
            } else {
                panic!("return from exec without err");
            }
        } else {
            io::Error::new(io::ErrorKind::NotFound, "")
        }
    }
