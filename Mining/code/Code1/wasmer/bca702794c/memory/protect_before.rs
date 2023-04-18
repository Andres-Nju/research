    pub unsafe fn protect(
        &mut self,
        range: impl RangeBounds<usize>,
        protect: Protect,
    ) -> Result<(), String> {
        let protect = protect.to_protect_const();

        let range_start = match range.start_bound() {
            Bound::Included(start) => *start,
            Bound::Excluded(start) => *start,
            Bound::Unbounded => 0,
        };

        let range_end = match range.end_bound() {
            Bound::Included(end) => *end,
            Bound::Excluded(end) => *end,
            Bound::Unbounded => self.size(),
        };

        let page_size = page_size::get();
        let start = self
            .ptr
            .add(round_down_to_page_size(range_start, page_size));
        let size = round_up_to_page_size(range_end - range_start, page_size);
        assert!(size <= self.size);

        // Commit the virtual memory.
        let ptr = VirtualAlloc(start as _, size, MEM_COMMIT, protect);

        if ptr.is_null() {
            Err("unable to protect memory".to_string())
        } else {
            self.protection = protection;
            Ok(())
        }
    }
