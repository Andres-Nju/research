    pub unsafe fn deallocate(ptr: *mut u8, _old_size: usize, align: usize) {
        if align <= MIN_ALIGN {
            let err = HeapFree(GetProcessHeap(), 0, ptr as LPVOID);
            debug_assert!(err != 0, "Failed to free heap memory: {}", GetLastError());
        } else {
            let header = get_header(ptr);
            let err = HeapFree(GetProcessHeap(), 0, header.0 as LPVOID);
            debug_assert!(err != 0, "Failed to free heap memory: {}", GetLastError());
        }
    }
