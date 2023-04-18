    pub fn emit(&mut self) {
        if self.cancelled() {
            return;
        }

        match self.level {
            Level::Bug |
            Level::Fatal |
            Level::PhaseFatal |
            Level::Error => {
                self.handler.bump_err_count();
            }

            Level::Warning |
            Level::Note |
            Level::Help |
            Level::Cancelled => {
            }
        }

        self.handler.emitter.borrow_mut().emit(&self);
        self.cancel();
        self.handler.panic_if_treat_err_as_bug();

        // if self.is_fatal() {
        //     panic!(FatalError);
        // }
    }
