    pub unsafe fn new(inner: NonNull<dyn UnsafeWake>) -> Self {
        Waker { inner: inner }
    }
