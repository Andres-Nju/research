    pub fn with_emitter_and_flags(e: Box<dyn Emitter + sync::Send>, flags: HandlerFlags) -> Handler
    {
        Handler {
            flags,
            err_count: AtomicUsize::new(0),
            emitter: Lock::new(e),
            continue_after_error: LockCell::new(true),
            delayed_span_bugs: Lock::new(Vec::new()),
            taught_diagnostics: Lock::new(FxHashSet()),
            emitted_diagnostic_codes: Lock::new(FxHashSet()),
            emitted_diagnostics: Lock::new(FxHashSet()),
        }
    }
