pub fn socketpair_impl() -> Result<(FileDescriptor, FileDescriptor)> {
    let mut fds = [-1i32; 2];
    #[cfg(target_os = "illumos")]
    let domain = libc::AF_UNIX;
    #[cfg(not(target_os = "illumos"))]
    let domain = libc::PF_LOCAL;

    let res = unsafe { libc::socketpair(domain, libc::SOCK_STREAM, 0, fds.as_mut_ptr()) };
    if res == -1 {
        Err(Error::Socketpair(std::io::Error::last_os_error()))
    } else {
        let mut read = FileDescriptor {
            handle: OwnedHandle {
                handle: fds[0],
                handle_type: (),
            },
        };
        let mut write = FileDescriptor {
            handle: OwnedHandle {
                handle: fds[1],
                handle_type: (),
            },
        };
        read.handle.cloexec()?;
        write.handle.cloexec()?;
        Ok((read, write))
    }
}
