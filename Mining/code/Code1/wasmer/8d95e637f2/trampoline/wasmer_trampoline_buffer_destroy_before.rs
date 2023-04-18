pub unsafe extern "C" fn wasmer_trampoline_buffer_destroy(buffer: *mut wasmer_trampoline_buffer_t) {
    if !buffer.is_null() {
        Box::from_raw(buffer);
    }
}
