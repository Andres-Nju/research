fn set_controlling_terminal(fd: c_int) {
    let res = unsafe {
        // Cross platform issue because on linux this is u64 as u64 (does nothing)
        // But on macos this is u32 as u64, asking for u64::from(u32)
        #[cfg_attr(feature = "clippy", allow(cast_lossless))]
        libc::ioctl(fd, TIOCSCTTY as u64, 0)
    };

    if res < 0 {
        die!("ioctl TIOCSCTTY failed: {}", errno());
    }
}
