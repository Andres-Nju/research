    pub fn swap(&mut self, a: usize, b: usize) {
        debug_assert_eq!(std::mem::size_of::<Row<T>>(), 32);

        let a = self.compute_index(a);
        let b = self.compute_index(b);

        unsafe {
            // Cast to a qword array to opt out of copy restrictions and avoid
            // drop hazards. Byte array is no good here since for whatever
            // reason LLVM won't optimized it.
            let a_ptr = self.inner.as_mut_ptr().add(a) as *mut usize;
            let b_ptr = self.inner.as_mut_ptr().add(b) as *mut usize;

            // Copy 1 qword at a time.
            //
            // The optimizer unrolls this loop and vectorizes it.
            let mut tmp: usize;
            for i in 0..4 {
                tmp = *a_ptr.offset(i);
                *a_ptr.offset(i) = *b_ptr.offset(i);
                *b_ptr.offset(i) = tmp;
            }
        }
    }
