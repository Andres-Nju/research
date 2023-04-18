    fn getgrouplist(
        name: *const c_char,
        gid: gid_t,
        groups: *mut gid_t,
        ngroups: *mut c_int,
    ) -> c_int;
}

/// From: https://man7.org/linux/man-pages/man2/getgroups.2.html
/// > getgroups() returns the supplementary group IDs of the calling
/// > process in list.
/// > If size is zero, list is not modified, but the total number of
/// > supplementary group IDs for the process is returned.  This allows
/// > the caller to determine the size of a dynamically allocated list
/// > to be used in a further call to getgroups().
#[cfg(not(target_os = "redox"))]
pub fn get_groups() -> IOResult<Vec<gid_t>> {
    let mut groups = Vec::new();
    loop {
        let ngroups = match unsafe { getgroups(0, ptr::null_mut()) } {
            -1 => return Err(IOError::last_os_error()),
            // Not just optimization; 0 would mess up the next call
            0 => return Ok(Vec::new()),
            n => n,
        };

        // This is a small buffer, so we can afford to zero-initialize it and
        // use safe Vec operations
        groups.resize(ngroups.try_into().unwrap(), 0);
        let res = unsafe { getgroups(ngroups, groups.as_mut_ptr()) };
        if res == -1 {
            let err = IOError::last_os_error();
            if err.raw_os_error() == Some(libc::EINVAL) {
                // Number of groups changed, retry
                continue;
            } else {
                return Err(err);
            }
        } else {
            groups.truncate(ngroups.try_into().unwrap());
            return Ok(groups);
        }
    }
}
