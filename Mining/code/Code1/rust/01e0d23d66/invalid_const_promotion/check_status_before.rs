fn check_status(status: std::process::ExitStatus)
{
    use libc;
    use std::os::unix::process::ExitStatusExt;

    assert!(status.signal() == Some(libc::SIGILL)
            || status.signal() == Some(libc::SIGABRT));
}
