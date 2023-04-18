pub fn resolve_symname<F>(frame: Frame,
                          callback: F,
                          _: &BacktraceContext) -> io::Result<()>
    where F: FnOnce(Option<&str>) -> io::Result<()>
{
    unsafe {
        let mut info: Dl_info = intrinsics::init();
        let symname = if dladdr(frame.exact_position, &mut info) == 0 {
            None
        } else {
            CStr::from_ptr(info.dli_sname).to_str().ok()
        };
        callback(symname)
    }
}
