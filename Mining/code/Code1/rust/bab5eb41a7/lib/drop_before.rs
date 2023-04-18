    fn drop(&mut self) {
        if self.err_count() == 0 {
            let mut bugs = self.delayed_span_bug.borrow_mut();
            let has_bugs = !bugs.is_empty();
            for bug in bugs.drain(..) {
                DiagnosticBuilder::new_diagnostic(self, bug).emit();
            }
            if has_bugs {
                panic!("no errors encountered even though `delay_span_bug` issued");
            }
        }
    }
