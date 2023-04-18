    pub unsafe fn read(&self) {
        let r = libc::pthread_rwlock_rdlock(self.inner.get());

        // According to the pthread_rwlock_rdlock spec, this function **may**
        // fail with EDEADLK if a deadlock is detected. On the other hand
        // pthread mutexes will *never* return EDEADLK if they are initialized
        // as the "fast" kind (which ours always are). As a result, a deadlock
        // situation may actually return from the call to pthread_rwlock_rdlock
        // instead of blocking forever (as mutexes and Windows rwlocks do). Note
        // that not all unix implementations, however, will return EDEADLK for
        // their rwlocks.
        //
        // We roughly maintain the deadlocking behavior by panicking to ensure
        // that this lock acquisition does not succeed.
        //
        // We also check whether this lock is already write locked. This
        // is only possible if it was write locked by the current thread and
        // the implementation allows recursive locking. The POSIX standard
        // doesn't require recursively locking a rwlock to deadlock, but we can't
        // allow that because it could lead to aliasing issues.
        if r == libc::EAGAIN {
            panic!("rwlock maximum reader count exceeded");
        } else if r == libc::EDEADLK || *self.write_locked.get() {
            if r == 0 {
                self.raw_unlock();
            }
            panic!("rwlock read lock would result in deadlock");
        } else {
            debug_assert_eq!(r, 0);
            self.num_readers.fetch_add(1, Ordering::Relaxed);
        }
    }
