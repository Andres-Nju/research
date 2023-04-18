    pub unsafe fn current() -> Option<usize> {
        let mut ret = None;
        let mut attr: libc::pthread_attr_t = ::mem::zeroed();
        assert_eq!(libc::pthread_attr_init(&mut attr), 0);
        #[cfg(target_os = "freebsd")]
            let e = libc::pthread_attr_get_np(libc::pthread_self(), &mut attr);
        #[cfg(not(target_os = "freebsd"))]
            let e = libc::pthread_getattr_np(libc::pthread_self(), &mut attr);
        if e == 0 {
            let mut guardsize = 0;
            assert_eq!(libc::pthread_attr_getguardsize(&attr, &mut guardsize), 0);
            if guardsize == 0 {
                panic!("there is no guard page");
            }
            let mut stackaddr = ::ptr::null_mut();
            let mut size = 0;
            assert_eq!(libc::pthread_attr_getstack(&attr, &mut stackaddr,
                                                   &mut size), 0);

            ret = if cfg!(target_os = "freebsd") {
                Some(stackaddr as usize - guardsize as usize)
            } else if cfg!(target_os = "netbsd") {
                Some(stackaddr as usize)
            } else {
                Some(stackaddr as usize + guardsize as usize)
            };
        }
        assert_eq!(libc::pthread_attr_destroy(&mut attr), 0);
        ret
    }
}
