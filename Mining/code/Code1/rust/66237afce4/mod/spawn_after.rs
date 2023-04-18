    pub fn spawn<F, T>(self, f: F) -> io::Result<JoinHandle<T>> where
        F: FnOnce() -> T, F: Send + 'static, T: Send + 'static
    {
        let Builder { name, stack_size } = self;

        let stack_size = stack_size.unwrap_or(util::min_stack());

        let my_thread = Thread::new(name);
        let their_thread = my_thread.clone();

        let my_packet : Arc<UnsafeCell<Option<Result<T>>>>
            = Arc::new(UnsafeCell::new(None));
        let their_packet = my_packet.clone();

        let main = move || {
            if let Some(name) = their_thread.cname() {
                imp::Thread::set_name(name);
            }
            unsafe {
                thread_info::set(imp::guard::current(), their_thread);
                #[cfg(feature = "backtrace")]
                let try_result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
                    ::sys_common::backtrace::__rust_begin_short_backtrace(f)
                }));
                #[cfg(not(feature = "backtrace"))]
                let try_result = panic::catch_unwind(panic::AssertUnwindSafe(f));
                *their_packet.get() = Some(try_result);
            }
        };

        Ok(JoinHandle(JoinInner {
            native: unsafe {
                Some(imp::Thread::new(stack_size, Box::new(main))?)
            },
            thread: my_thread,
            packet: Packet(my_packet),
        }))
    }
