pub fn ___syscall140(ctx: &mut Ctx, _which: i32, mut varargs: VarArgs) -> i32 {
    // -> c_int
    debug!("emscripten::___syscall140 (lseek) {}", _which);
    let fd: i32 = varargs.get(ctx);
    let _offset_high: i32 = varargs.get(ctx); // We don't use the offset high as emscripten skips it
    let offset_low: i32 = varargs.get(ctx);
    let result_ptr_value: WasmPtr<i64> = varargs.get(ctx);
    let whence: i32 = varargs.get(ctx);
    let offset = offset_low as off_t;
    let ret = unsafe { lseek(fd, offset, whence) as i64 };

    let result_ptr = result_ptr_value.deref(ctx.memory(0)).unwrap();
    result_ptr.set(ret);

    debug!(
        "=> fd: {}, offset: {}, result: {}, whence: {} = {}\nlast os error: {}",
        fd,
        offset,
        ret,
        whence,
        0,
        Error::last_os_error(),
    );
    0
}
