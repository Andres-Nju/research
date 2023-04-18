    pub fn wait(&self, mutex: &Mutex) {
        unsafe {
            let lock = self.lock.get();
            let seq = self.seq.get();

            if *lock != mutex.lock.get() {
                if *lock != ptr::null_mut() {
                    panic!("Condvar used with more than one Mutex");
                }

                atomic_cxchg(lock as *mut usize, 0, mutex.lock.get() as usize);
            }

            mutex_unlock(*lock);

            let _ = futex(seq, FUTEX_WAIT, *seq, 0, ptr::null_mut());

            while atomic_xchg(*lock, 2) != 0 {
                let _ = futex(*lock, FUTEX_WAIT, 2, 0, ptr::null_mut());
            }
        }
    }

    #[inline]
