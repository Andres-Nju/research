pub fn read2(p1: AnonPipe,
             v1: &mut Vec<u8>,
             p2: AnonPipe,
             v2: &mut Vec<u8>) -> io::Result<()> {
    //FIXME: Use event based I/O multiplexing
    //unimplemented!()

    p1.read_to_end(v1)?;
    p2.read_to_end(v2)?;

    Ok(())

    /*
    // Set both pipes into nonblocking mode as we're gonna be reading from both
    // in the `select` loop below, and we wouldn't want one to block the other!
    let p1 = p1.into_fd();
    let p2 = p2.into_fd();
    p1.set_nonblocking(true)?;
    p2.set_nonblocking(true)?;

    loop {
        // wait for either pipe to become readable using `select`
        cvt_r(|| unsafe {
            let mut read: libc::fd_set = mem::zeroed();
            libc::FD_SET(p1.raw(), &mut read);
            libc::FD_SET(p2.raw(), &mut read);
            libc::select(max + 1, &mut read, ptr::null_mut(), ptr::null_mut(),
                         ptr::null_mut())
        })?;

        // Read as much as we can from each pipe, ignoring EWOULDBLOCK or
        // EAGAIN. If we hit EOF, then this will happen because the underlying
        // reader will return Ok(0), in which case we'll see `Ok` ourselves. In
        // this case we flip the other fd back into blocking mode and read
        // whatever's leftover on that file descriptor.
        let read = |fd: &FileDesc, dst: &mut Vec<u8>| {
            match fd.read_to_end(dst) {
                Ok(_) => Ok(true),
                Err(e) => {
                    if e.raw_os_error() == Some(libc::EWOULDBLOCK) ||
                       e.raw_os_error() == Some(libc::EAGAIN) {
                        Ok(false)
                    } else {
                        Err(e)
                    }
                }
            }
        };
        if read(&p1, v1)? {
            p2.set_nonblocking(false)?;
            return p2.read_to_end(v2).map(|_| ());
        }
        if read(&p2, v2)? {
            p1.set_nonblocking(false)?;
            return p1.read_to_end(v1).map(|_| ());
        }
    }
    */
}
