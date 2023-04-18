pub fn get_task_waker<F, R>(f: F) -> R
where
    F: FnOnce(&LocalWaker) -> R
{
    let waker_ptr = TLS_WAKER.with(|tls_waker| {
        // Clear the entry so that nested `get_task_waker` calls
        // will fail or set their own value.
        tls_waker.replace(None)
    });
    let _reset_waker = SetOnDrop(waker_ptr);

    let waker_ptr = waker_ptr.expect(
        "TLS LocalWaker not set. This is a rustc bug. \
        Please file an issue on https://github.com/rust-lang/rust.");
    unsafe { f(waker_ptr.as_ref()) }
}
