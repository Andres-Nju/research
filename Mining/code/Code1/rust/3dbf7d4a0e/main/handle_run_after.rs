fn handle_run(socket: TcpStream, work: &Path, lock: &Mutex<()>) {
    let mut arg = Vec::new();
    let mut reader = BufReader::new(socket);

    // Allocate ourselves a directory that we'll delete when we're done to save
    // space.
    let n = TEST.fetch_add(1, Ordering::SeqCst);
    let path = work.join(format!("test{}", n));
    t!(fs::create_dir(&path));
    let _a = RemoveOnDrop { inner: &path };

    // First up we'll get a list of arguments delimited with 0 bytes. An empty
    // argument means that we're done.
    let mut args = Vec::new();
    while t!(reader.read_until(0, &mut arg)) > 1 {
        args.push(t!(str::from_utf8(&arg[..arg.len() - 1])).to_string());
        arg.truncate(0);
    }

    // Next we'll get a bunch of env vars in pairs delimited by 0s as well
    let mut env = Vec::new();
    arg.truncate(0);
    while t!(reader.read_until(0, &mut arg)) > 1 {
        let key_len = arg.len() - 1;
        let val_len = t!(reader.read_until(0, &mut arg)) - 1;
        {
            let key = &arg[..key_len];
            let val = &arg[key_len + 1..][..val_len];
            let key = t!(str::from_utf8(key)).to_string();
            let val = t!(str::from_utf8(val)).to_string();
            env.push((key, val));
        }
        arg.truncate(0);
    }

    // The section of code from here down to where we drop the lock is going to
    // be a critical section for us. On Linux you can't execute a file which is
    // open somewhere for writing, as you'll receive the error "text file busy".
    // Now here we never have the text file open for writing when we spawn it,
    // so why do we still need a critical section?
    //
    // Process spawning first involves a `fork` on Unix, which clones all file
    // descriptors into the child process. This means that it's possible for us
    // to open the file for writing (as we're downloading it), then some other
    // thread forks, then we close the file and try to exec. At that point the
    // other thread created a child process with the file open for writing, and
    // we attempt to execute it, so we get an error.
    //
    // This race is resolve by ensuring that only one thread can write the file
    // and spawn a child process at once. Kinda an unfortunate solution, but we
    // don't have many other choices with this sort of setup!
    //
    // In any case the lock is acquired here, before we start writing any files.
    // It's then dropped just after we spawn the child. That way we don't lock
    // the execution of the child, just the creation of its files.
    let lock = lock.lock();

    // Next there's a list of dynamic libraries preceded by their filenames.
    while t!(reader.fill_buf())[0] != 0 {
        recv(&path, &mut reader);
    }
    assert_eq!(t!(reader.read(&mut [0])), 1);

    // Finally we'll get the binary. The other end will tell us how big the
    // binary is and then we'll download it all to the exe path we calculated
    // earlier.
    let exe = recv(&path, &mut reader);

    let mut cmd = Command::new(&exe);
    for arg in args {
        cmd.arg(arg);
    }
    for (k, v) in env {
        cmd.env(k, v);
    }

    // Support libraries were uploaded to `work` earlier, so make sure that's
    // in `LD_LIBRARY_PATH`. Also include our own current dir which may have
    // had some libs uploaded.
    cmd.env("LD_LIBRARY_PATH",
            format!("{}:{}", work.display(), path.display()));

    // Spawn the child and ferry over stdout/stderr to the socket in a framed
    // fashion (poor man's style)
    let mut child = t!(cmd.stdin(Stdio::null())
                          .stdout(Stdio::piped())
                          .stderr(Stdio::piped())
                          .spawn());
    drop(lock);
    let mut stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    let socket = Arc::new(Mutex::new(reader.into_inner()));
    let socket2 = socket.clone();
    let thread = thread::spawn(move || my_copy(&mut stdout, 0, &*socket2));
    my_copy(&mut stderr, 1, &*socket);
    thread.join().unwrap();

    // Finally send over the exit status.
    let status = t!(child.wait());
    let (which, code) = match status.code() {
        Some(n) => (0, n),
        None => (1, status.signal().unwrap()),
    };
    t!(socket.lock().unwrap().write_all(&[
        which,
        (code >> 24) as u8,
        (code >> 16) as u8,
        (code >>  8) as u8,
        (code >>  0) as u8,
    ]));
}
