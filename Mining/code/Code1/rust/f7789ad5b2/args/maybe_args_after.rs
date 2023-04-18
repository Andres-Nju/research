fn maybe_args() -> Option<Vec<OsString>> {
    unsafe {
        let (argc, buf_size) = wasi::args_sizes_get().ok()?;
        let mut argv = Vec::with_capacity(argc);
        let mut buf = Vec::with_capacity(buf_size);
        wasi::args_get(argv.as_mut_ptr(), buf.as_mut_ptr()).ok()?;
        argv.set_len(argc);
        let mut ret = Vec::with_capacity(argc);
        for ptr in argv {
            let s = CStr::from_ptr(ptr.cast());
            ret.push(OsStr::from_bytes(s.to_bytes()).to_owned());
        }
        Some(ret)
    }
}
