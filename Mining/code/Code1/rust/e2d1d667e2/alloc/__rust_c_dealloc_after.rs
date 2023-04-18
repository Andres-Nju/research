pub unsafe extern "C" fn __rust_c_dealloc(ptr: *mut u8, size: usize, align: usize) {
    crate::alloc::dealloc(ptr, Layout::from_size_align_unchecked(size, align))
}
