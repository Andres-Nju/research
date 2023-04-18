pub fn ___syscall196(ctx: &mut Ctx, _which: i32, mut varargs: VarArgs) -> i32 {
    debug!("emscripten::___syscall196 (lstat64) {}", _which);
    let path_ptr: c_int = varargs.get(ctx);
    let buf_ptr: u32 = varargs.get(ctx);
    let path = emscripten_memory_pointer!(ctx.memory(0), path_ptr) as *const i8;
    unsafe {
        let mut stat: stat = std::mem::zeroed();

        #[cfg(target_os = "macos")]
        let stat_ptr = &mut stat as *mut stat as *mut c_void;
        #[cfg(not(target_os = "macos"))]
        let stat_ptr = &mut stat as *mut stat;

        #[cfg(target_os = "macos")]
        let ret = lstat64(path, stat_ptr);
        #[cfg(not(target_os = "macos"))]
        let ret = lstat(path, stat_ptr);

        debug!("ret: {}", ret);
        if ret != 0 {
            return ret;
        }
        utils::copy_stat_into_wasm(ctx, buf_ptr, &stat);
    }
    0
}
