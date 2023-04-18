    pub fn report_an_error(&self, error_info: ErrorInfo, value: HandleValue) {
        // Step 1.
        if self.in_error_reporting_mode.get() {
            return;
        }

        // Step 2.
        self.in_error_reporting_mode.set(true);

        // Steps 3-12.
        // FIXME(#13195): muted errors.
        let event = ErrorEvent::new(GlobalRef::Window(self),
                                    atom!("error"),
                                    EventBubbles::DoesNotBubble,
                                    EventCancelable::Cancelable,
                                    error_info.message.into(),
                                    error_info.filename.into(),
                                    error_info.lineno,
                                    error_info.column,
                                    value);

        // Step 13.
        event.upcast::<Event>().fire(self.upcast::<EventTarget>());

        // Step 14.
        self.in_error_reporting_mode.set(false);
    }
