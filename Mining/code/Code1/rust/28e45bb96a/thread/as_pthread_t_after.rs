    fn as_pthread_t(&self) -> RawPthread;
    /// Consumes the thread, returning the raw pthread_t
    ///
    /// This function **transfers ownership** of the underlying pthread_t to
    /// the caller. Callers are then the unique owners of the pthread_t and
    /// must either detach or join the pthread_t once it's no longer needed.
    fn into_pthread_t(self) -> RawPthread;
}

#[unstable(feature = "thread_extensions", issue = "29791")]
impl<T> JoinHandleExt for JoinHandle<T> {
    fn as_pthread_t(&self) -> RawPthread {
        self.as_inner().id() as RawPthread
    }
    fn into_pthread_t(self) -> RawPthread {
        self.into_inner().into_id() as RawPthread
    }
}
