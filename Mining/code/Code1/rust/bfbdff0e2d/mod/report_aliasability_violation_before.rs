    pub fn report_aliasability_violation(&self,
                                         span: Span,
                                         kind: AliasableViolationKind,
                                         cause: mc::AliasableReason) {
        let mut is_closure = false;
        let prefix = match kind {
            MutabilityViolation => {
                "cannot assign to data"
            }
            BorrowViolation(euv::ClosureCapture(_)) |
            BorrowViolation(euv::OverloadedOperator) |
            BorrowViolation(euv::AddrOf) |
            BorrowViolation(euv::AutoRef) |
            BorrowViolation(euv::AutoUnsafe) |
            BorrowViolation(euv::RefBinding) |
            BorrowViolation(euv::MatchDiscriminant) => {
                "cannot borrow data mutably"
            }

            BorrowViolation(euv::ClosureInvocation) => {
                is_closure = true;
                "closure invocation"
            }

            BorrowViolation(euv::ForLoop) => {
                "`for` loop"
            }
        };

        let mut err = match cause {
            mc::AliasableOther => {
                struct_span_err!(
                    self.tcx.sess, span, E0385,
                    "{} in an aliasable location", prefix)
            }
            mc::AliasableReason::UnaliasableImmutable => {
                struct_span_err!(
                    self.tcx.sess, span, E0386,
                    "{} in an immutable container", prefix)
            }
            mc::AliasableClosure(id) => {
                let mut err = struct_span_err!(
                    self.tcx.sess, span, E0387,
                    "{} in a captured outer variable in an `Fn` closure", prefix);
                if let BorrowViolation(euv::ClosureCapture(_)) = kind {
                    // The aliasability violation with closure captures can
                    // happen for nested closures, so we know the enclosing
                    // closure incorrectly accepts an `Fn` while it needs to
                    // be `FnMut`.
                    span_help!(&mut err, self.tcx.map.span(id),
                           "consider changing this to accept closures that implement `FnMut`");
                } else {
                    span_help!(&mut err, self.tcx.map.span(id),
                           "consider changing this closure to take self by mutable reference");
                }
                err
            }
            mc::AliasableStatic |
            mc::AliasableStaticMut => {
                struct_span_err!(
                    self.tcx.sess, span, E0388,
                    "{} in a static location", prefix)
            }
            mc::AliasableBorrowed => {
                struct_span_err!(
                    self.tcx.sess, span, E0389,
                    "{} in a `&` reference", prefix)
            }
        };

        if is_closure {
            err.help("closures behind references must be called via `&mut`");
        }
        err.emit();
    }
