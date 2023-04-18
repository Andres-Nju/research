extern "C" fn exception_cleanup(ptr: *mut libc::c_void) -> DestructorRet {
    unsafe {
        if let Some(b) = (ptr as *mut Exception).read().data {
            drop(b);
            super::__rust_drop_panic();
        }
        #[cfg(any(target_arch = "arm", target_arch = "wasm32"))]
        ptr
    }
}
