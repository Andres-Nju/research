    fn check_and_note_conflicting_crates(&self,
                                         err: &mut DiagnosticBuilder,
                                         terr: &TypeError<'tcx>,
                                         sp: Span) {
        let report_path_match = |err: &mut DiagnosticBuilder, did1: DefId, did2: DefId| {
            // Only external crates, if either is from a local
            // module we could have false positives
            if !(did1.is_local() || did2.is_local()) && did1.krate != did2.krate {
                let exp_path = self.tcx.item_path_str(did1);
                let found_path = self.tcx.item_path_str(did2);
                // We compare strings because DefPath can be different
                // for imported and non-imported crates
                if exp_path == found_path {
                    let crate_name = self.tcx.sess.cstore.crate_name(did1.krate);
                    err.span_note(sp, &format!("Perhaps two different versions \
                                                of crate `{}` are being used?",
                                               crate_name));
                }
            }
        };
        match *terr {
            TypeError::Sorts(ref exp_found) => {
                // if they are both "path types", there's a chance of ambiguity
                // due to different versions of the same crate
                match (&exp_found.expected.sty, &exp_found.found.sty) {
                    (&ty::TyAdt(exp_adt, _), &ty::TyAdt(found_adt, _)) => {
                        report_path_match(err, exp_adt.did, found_adt.did);
                    },
                    _ => ()
                }
            },
            TypeError::Traits(ref exp_found) => {
                report_path_match(err, exp_found.expected, exp_found.found);
            },
            _ => () // FIXME(#22750) handle traits and stuff
        }
    }

    fn note_error_origin(&self,
                         err: &mut DiagnosticBuilder<'tcx>,
                         origin: &TypeOrigin)
    {
        match origin {
            &TypeOrigin::MatchExpressionArm(_, arm_span, source) => match source {
                hir::MatchSource::IfLetDesugar {..} => {
                    err.span_note(arm_span, "`if let` arm with an incompatible type");
                }
                _ => {
                    err.span_note(arm_span, "match arm with an incompatible type");
                }
            },
            _ => ()
        }
    }

    pub fn note_type_err(&self,
                         diag: &mut DiagnosticBuilder<'tcx>,
                         origin: TypeOrigin,
                         secondary_span: Option<(Span, String)>,
                         values: Option<ValuePairs<'tcx>>,
                         terr: &TypeError<'tcx>)
    {
        let expected_found = match values {
            None => None,
            Some(values) => match self.values_str(&values) {
                Some((expected, found)) => Some((expected, found)),
                None => {
                    // Derived error. Cancel the emitter.
                    self.tcx.sess.diagnostic().cancel(diag);
                    return
                }
            }
        };

        let span = origin.span();

        if let Some((expected, found)) = expected_found {
            let is_simple_error = if let &TypeError::Sorts(ref values) = terr {
                values.expected.is_primitive() && values.found.is_primitive()
            } else {
                false
            };

            if !is_simple_error {
                if expected == found {
                    if let &TypeError::Sorts(ref values) = terr {
                        diag.note_expected_found_extra(
                            &"type", &expected, &found,
                            &format!(" ({})", values.expected.sort_string(self.tcx)),
                            &format!(" ({})", values.found.sort_string(self.tcx)));
                    } else {
                        diag.note_expected_found(&"type", &expected, &found);
                    }
                } else {
                    diag.note_expected_found(&"type", &expected, &found);
                }
            }
        }

        diag.span_label(span, &terr);
        if let Some((sp, msg)) = secondary_span {
            diag.span_label(sp, &msg);
        }

        self.note_error_origin(diag, &origin);
        self.check_and_note_conflicting_crates(diag, terr, span);
        self.tcx.note_and_explain_type_err(diag, terr, span);
    }

    pub fn report_and_explain_type_error(&self,
                                         trace: TypeTrace<'tcx>,
                                         terr: &TypeError<'tcx>)
                                         -> DiagnosticBuilder<'tcx>
    {
        let span = trace.origin.span();
        let failure_str = trace.origin.as_failure_str();
        let mut diag = match trace.origin {
            // FIXME: use distinct codes for each case
            TypeOrigin::IfExpressionWithNoElse(_) => {
                struct_span_err!(self.tcx.sess, span, E0317, "{}", failure_str)
            },
            _ => {
                struct_span_err!(self.tcx.sess, span, E0308, "{}", failure_str)
            },
        };
        self.note_type_err(&mut diag, trace.origin, None, Some(trace.values), terr);
        diag
    }

    /// Returns a string of the form "expected `{}`, found `{}`".
    fn values_str(&self, values: &ValuePairs<'tcx>) -> Option<(String, String)> {
        match *values {
            infer::Types(ref exp_found) => self.expected_found_str(exp_found),
            infer::TraitRefs(ref exp_found) => self.expected_found_str(exp_found),
            infer::PolyTraitRefs(ref exp_found) => self.expected_found_str(exp_found),
        }
    }
