    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        ty::tls::with(|tcx| {
            if tcx.def_key(self.instance.def_id()).disambiguated_data.data
                == DefPathData::ClosureExpr
            {
                write!(f, "inside call to closure")?;
            } else {
                write!(f, "inside call to `{}`", self.instance)?;
            }
            if !self.call_site.is_dummy() {
                let lo = tcx.sess.source_map().lookup_char_pos(self.call_site.lo());
                write!(f, " at {}:{}:{}", lo.file.name, lo.line, lo.col.to_usize() + 1)?;
            }
            Ok(())
        })
    }
}

impl<'tcx> ConstEvalErr<'tcx> {
    pub fn struct_error(
        &self,
        tcx: TyCtxtAt<'tcx>,
        message: &str,
        emit: impl FnOnce(DiagnosticBuilder<'_>),
    ) -> Result<(), ErrorHandled> {
        self.struct_generic(tcx, message, emit, None)
    }

    pub fn report_as_error(&self, tcx: TyCtxtAt<'tcx>, message: &str) -> ErrorHandled {
        match self.struct_error(tcx, message, |mut e| e.emit()) {
            Ok(_) => ErrorHandled::Reported,
            Err(x) => x,
        }
    }

    pub fn report_as_lint(
        &self,
        tcx: TyCtxtAt<'tcx>,
        message: &str,
        lint_root: hir::HirId,
        span: Option<Span>,
    ) -> ErrorHandled {
        match self.struct_generic(
            tcx,
            message,
            |mut lint: DiagnosticBuilder<'_>| {
                // Apply the span.
                if let Some(span) = span {
                    let primary_spans = lint.span.primary_spans().to_vec();
                    // point at the actual error as the primary span
                    lint.replace_span_with(span);
                    // point to the `const` statement as a secondary span
                    // they don't have any label
                    for sp in primary_spans {
                        if sp != span {
                            lint.span_label(sp, "");
                        }
                    }
                }
                lint.emit();
            },
            Some(lint_root),
        ) {
            Ok(_) => ErrorHandled::Reported,
            Err(err) => err,
        }
    }

    /// Sets the message passed in via `message` and adds span labels before handing control back
    /// to `emit` to do any final processing. It's the caller's responsibility to call emit(),
    /// stash(), etc. within the `emit` function to dispose of the diagnostic properly.
    fn struct_generic(
        &self,
        tcx: TyCtxtAt<'tcx>,
        message: &str,
        emit: impl FnOnce(DiagnosticBuilder<'_>),
        lint_root: Option<hir::HirId>,
    ) -> Result<(), ErrorHandled> {
        let must_error = match self.error {
            InterpError::MachineStop(_) => bug!("CTFE does not stop"),
            err_inval!(Layout(LayoutError::Unknown(_))) | err_inval!(TooGeneric) => {
                return Err(ErrorHandled::TooGeneric);
            }
            err_inval!(TypeckError) => return Err(ErrorHandled::Reported),
            err_inval!(Layout(LayoutError::SizeOverflow(_))) => true,
            _ => false,
        };
        trace!("reporting const eval failure at {:?}", self.span);

        let add_span_labels = |err: &mut DiagnosticBuilder<'_>| {
            if !must_error {
                err.span_label(self.span, self.error.to_string());
            }
            // Skip the last, which is just the environment of the constant.  The stacktrace
            // is sometimes empty because we create "fake" eval contexts in CTFE to do work
            // on constant values.
            if self.stacktrace.len() > 0 {
                for frame_info in &self.stacktrace[..self.stacktrace.len() - 1] {
                    err.span_label(frame_info.call_site, frame_info.to_string());
                }
            }
        };

        if let (Some(lint_root), false) = (lint_root, must_error) {
            let hir_id = self
                .stacktrace
                .iter()
                .rev()
                .filter_map(|frame| frame.lint_root)
                .next()
                .unwrap_or(lint_root);
            tcx.struct_span_lint_hir(
                rustc_session::lint::builtin::CONST_ERR,
                hir_id,
                tcx.span,
                |lint| {
                    let mut err = lint.build(message);
                    add_span_labels(&mut err);
                    emit(err);
                },
            );
        } else {
            let mut err = if must_error {
                struct_error(tcx, &self.error.to_string())
            } else {
                struct_error(tcx, message)
            };
            add_span_labels(&mut err);
            emit(err);
        };
        Ok(())
    }
}

pub fn struct_error<'tcx>(tcx: TyCtxtAt<'tcx>, msg: &str) -> DiagnosticBuilder<'tcx> {
    struct_span_err!(tcx.sess, tcx.span, E0080, "{}", msg)
}

/// Packages the kind of error we got from the const code interpreter
/// up with a Rust-level backtrace of where the error occurred.
/// Thsese should always be constructed by calling `.into()` on
/// a `InterpError`. In `librustc_mir::interpret`, we have `throw_err_*`
/// macros for this.
#[derive(Debug)]
pub struct InterpErrorInfo<'tcx> {
    pub kind: InterpError<'tcx>,
    backtrace: Option<Box<Backtrace>>,
}
