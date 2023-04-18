        unsafe fn rwlock_new(init: &mut MaybeUninit<RWLock>) {
            init.set(RWLock::new());
        }
