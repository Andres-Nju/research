        unsafe fn rwlock_new(init: &mut MaybeUninit<RWLock>) {
            init.write(RWLock::new());
        }
