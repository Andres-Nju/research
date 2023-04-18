    fn from(local_waker: LocalWaker) -> Self {
        let inner = local_waker.inner;
        mem::forget(local_waker);
        Waker { inner }
    }
