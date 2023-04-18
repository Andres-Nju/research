    fn delay_as_bug(&self, diagnostic: Diagnostic) {
        if self.flags.report_delayed_bugs {
            DiagnosticBuilder::new_diagnostic(self, diagnostic.clone()).emit();
        }
        self.delayed_span_bug.borrow_mut().push(diagnostic);
    }
