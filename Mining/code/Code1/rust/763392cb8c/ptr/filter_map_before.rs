    pub fn filter_map<F>(mut self, f: F) -> Option<P<T>> where
        F: FnOnce(T) -> Option<T>,
    {
        let p: *mut T = &mut *self.ptr;

        // Leak self in case of panic.
        // FIXME(eddyb) Use some sort of "free guard" that
        // only deallocates, without dropping the pointee,
        // in case the call the `f` below ends in a panic.
        mem::forget(self);

        unsafe {
            if let Some(v) = f(ptr::read(p)) {
                ptr::write(p, v);

                // Recreate self from the raw pointer.
                Some(P { ptr: Box::from_raw(p) })
            } else {
                None
            }
        }
    }
