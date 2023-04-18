fn acquire(config: &Config,
           msg: &str,
           path: &Path,
           try: &Fn() -> io::Result<()>,
           block: &Fn() -> io::Result<()>) -> CargoResult<()> {

    // File locking on Unix is currently implemented via `flock`, which is known
    // to be broken on NFS. We could in theory just ignore errors that happen on
    // NFS, but apparently the failure mode [1] for `flock` on NFS is **blocking
    // forever**, even if the nonblocking flag is passed!
    //
    // As a result, we just skip all file locks entirely on NFS mounts. That
    // should avoid calling any `flock` functions at all, and it wouldn't work
    // there anyway.
    //
    // [1]: https://github.com/rust-lang/cargo/issues/2615
    if is_on_nfs_mount(path) {
        return Ok(())
    }

    match try() {
        Ok(()) => return Ok(()),

        // Like above, where we ignore file locking on NFS mounts on Linux, we
        // do the same on OSX here. Note that ENOTSUP is an OSX_specific
        // constant.
        #[cfg(target_os = "macos")]
        Err(ref e) if e.raw_os_error() == Some(libc::ENOTSUP) => return Ok(()),

        Err(e) => {
            if e.raw_os_error() != lock_contended_error().raw_os_error() {
                return Err(human(e)).chain_error(|| {
                    human(format!("failed to lock file: {}", path.display()))
                })
            }
        }
    }
    let msg = format!("waiting for file lock on {}", msg);
    config.shell().err().say_status("Blocking", &msg, CYAN, true)?;

    return block().chain_error(|| {
        human(format!("failed to lock file: {}", path.display()))
    });

    #[cfg(all(target_os = "linux", not(target_env = "musl")))]
    fn is_on_nfs_mount(path: &Path) -> bool {
        use std::ffi::CString;
        use std::mem;
        use std::os::unix::prelude::*;

        let path = match CString::new(path.as_os_str().as_bytes()) {
            Ok(path) => path,
            Err(_) => return false,
        };

        unsafe {
            let mut buf: libc::statfs = mem::zeroed();
            let r = libc::statfs(path.as_ptr(), &mut buf);

            r == 0 && buf.f_type as u32 == libc::NFS_SUPER_MAGIC as u32
        }
    }

    #[cfg(any(not(target_os = "linux"), target_env = "musl"))]
    fn is_on_nfs_mount(_path: &Path) -> bool {
        false
    }
}
