    pub(crate) fn free_handles(&mut self, handle: DualHandle) {
        let start = (handle.gpu.ptr - self.start.gpu.ptr) / self.handle_size;
        let handle_range = start..start + handle.size as u64;
        self.range_allocator.free_range(handle_range).expect("Heap free failed!  Handle passed in was invalid.");
    }
