    pub fn new_pair(fam: c_int, ty: c_int) -> io::Result<(Socket, Socket)> {
        unsafe {
            let mut fds = [0, 0];

            // Like above, see if we can set cloexec atomically
            if cfg!(linux) {
                match cvt(libc::socketpair(fam, ty | SOCK_CLOEXEC, 0, fds.as_mut_ptr())) {
                    Ok(_) => {
                        return Ok((Socket(FileDesc::new(fds[0])), Socket(FileDesc::new(fds[1]))));
                    }
                    Err(ref e) if e.raw_os_error() == Some(libc::EINVAL) => {},
                    Err(e) => return Err(e),
                }
            }

            cvt(libc::socketpair(fam, ty, 0, fds.as_mut_ptr()))?;
            let a = FileDesc::new(fds[0]);
            let b = FileDesc::new(fds[1]);
            a.set_cloexec()?;
            b.set_cloexec()?;
            Ok((Socket(a), Socket(b)))
        }
    }
