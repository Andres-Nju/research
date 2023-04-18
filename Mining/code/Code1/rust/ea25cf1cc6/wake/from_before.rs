    fn from(local_waker: LocalWaker) -> Self {
        Waker { inner: local_waker.inner }
    }
