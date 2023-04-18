extern "C" fn exception_cleanup(ptr: *mut libc::c_void) -> DestructorRet {
    unsafe {
        ptr::drop_in_place(ptr as *mut Exception);
        super::__rust_drop_panic();
    }
}
