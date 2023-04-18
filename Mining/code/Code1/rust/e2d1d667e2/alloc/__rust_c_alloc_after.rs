pub unsafe extern "C" fn __rust_c_alloc(size: usize, align: usize) -> *mut u8 {
    crate::alloc::alloc(Layout::from_size_align_unchecked(size, align))
}
