    pub fn into_diagnostic(self) -> Diagnostic<usize> {
        let d = match self.error {
            ProximateShellError::MissingValue { span, reason } => {
                let mut d = Diagnostic::bug().with_message(format!("Internal Error (missing value) :: {}", reason));

                if let Some(span) = span {
                    d = d.with_labels(vec![Label::primary(0, span)]);
                }

                d
            }
            ProximateShellError::ArgumentError {
                command,
                error,
            } => match error {
                ArgumentError::InvalidExternalWord => Diagnostic::error().with_message("Invalid bare word for Nu command (did you intend to invoke an external command?)")
                .with_labels(vec![Label::primary(0, command.span)]),
                ArgumentError::UnexpectedArgument(argument) => Diagnostic::error().with_message(
                    format!(
                        "{} unexpected {}",
                        Color::Cyan.paint(&command.item),
                        Color::Green.bold().paint(&argument.item)
                    )
                )
                .with_labels(
                    vec![Label::primary(0, argument.span).with_message(
                        format!("unexpected argument (try {} -h)", &command.item))]
                ),
                ArgumentError::UnexpectedFlag(flag) => Diagnostic::error().with_message(
                    format!(
                        "{} unexpected {}",
                        Color::Cyan.paint(&command.item),
                        Color::Green.bold().paint(&flag.item)
                    ),
                )
                .with_labels(vec![
                    Label::primary(0, flag.span).with_message(
                    format!("unexpected flag (try {} -h)", &command.item))
                    ]),
                ArgumentError::MissingMandatoryFlag(name) => Diagnostic::error().with_message(                    format!(
                        "{} requires {}{}",
                        Color::Cyan.paint(&command.item),
                        Color::Green.bold().paint("--"),
                        Color::Green.bold().paint(name)
                    ),
                )
                .with_labels(vec![Label::primary(0, command.span)]),
                ArgumentError::MissingMandatoryPositional(name) => Diagnostic::error().with_message(
                    format!(
                        "{} requires {} parameter",
                        Color::Cyan.paint(&command.item),
                        Color::Green.bold().paint(name.clone())
                    ),
                )
                .with_labels(
                    vec![Label::primary(0, command.span).with_message(format!("requires {} parameter", name))],
                ),
                ArgumentError::MissingValueForName(name) => Diagnostic::error().with_message(
                    format!(
                        "{} is missing value for flag {}{}",
                        Color::Cyan.paint(&command.item),
                        Color::Green.bold().paint("--"),
                        Color::Green.bold().paint(name)
                    ),
                )
                .with_labels(vec![Label::primary(0, command.span)]),
                ArgumentError::BadValue(msg) => Diagnostic::error().with_message(msg.clone()).with_labels(vec![Label::primary(0, command.span).with_message(msg)])
            },
            ProximateShellError::TypeError {
                expected,
                actual:
                    Spanned {
                        item: Some(actual),
                        span,
                    },
            } => Diagnostic::error().with_message("Type Error").with_labels(
                vec![Label::primary(0, span)
                    .with_message(format!("Expected {}, found {}", expected, actual))],
            ),
            ProximateShellError::TypeError {
                expected,
                actual:
                    Spanned {
                        item: None,
                        span
                    },
            } => Diagnostic::error().with_message("Type Error")
                .with_labels(vec![Label::primary(0, span).with_message(expected)]),

            ProximateShellError::UnexpectedEof {
                expected, span
            } => Diagnostic::error().with_message("Unexpected end of input")
                .with_labels(vec![Label::primary(0, span).with_message(format!("Expected {}", expected))]),

            ProximateShellError::RangeError {
                kind,
                operation,
                actual_kind:
                    Spanned {
                        item,
                        span
                    },
            } => Diagnostic::error().with_message("Range Error").with_labels(
                vec![Label::primary(0, span).with_message(format!(
                    "Expected to convert {} to {} while {}, but it was out of range",
                    item,
                    kind.display(),
                    operation
                ))],
            ),

            ProximateShellError::SyntaxError {
                problem:
                    Spanned {
                        span,
                        item
                    },
            } => Diagnostic::error().with_message("Syntax Error")
                .with_labels(vec![Label::primary(0, span).with_message(item)]),

            ProximateShellError::MissingProperty { subpath, expr, .. } => {

                let mut diag = Diagnostic::error().with_message("Missing property");

                if subpath.span == Span::unknown() {
                    diag.message = format!("Missing property (for {})", subpath.item);
                } else {
                    let subpath = Label::primary(0, subpath.span).with_message(subpath.item);
                    let mut labels = vec![subpath];

                    if expr.span != Span::unknown() {
                        let expr = Label::primary(0, expr.span).with_message(expr.item);
                        labels.push(expr);
                    }
                    diag = diag.with_labels(labels);
                }

                diag
            }

            ProximateShellError::InvalidIntegerIndex { subpath,integer } => {
                let mut diag = Diagnostic::error().with_message("Invalid integer property");
                let mut labels = vec![];
                if subpath.span == Span::unknown() {
                    diag.message = format!("Invalid integer property (for {})", subpath.item)
                } else {
                    let label = Label::primary(0, subpath.span).with_message(subpath.item);
                    labels.push(label);
                }

                labels.push(Label::secondary(0, integer).with_message("integer"));
                diag = diag.with_labels(labels);

                diag
            }

            ProximateShellError::Diagnostic(diag) => diag.diagnostic,
            ProximateShellError::CoerceError { left, right } => {
                Diagnostic::error().with_message("Coercion error")
                    .with_labels(vec![Label::primary(0, left.span).with_message(left.item),
                    Label::secondary(0, right.span).with_message(right.item)])
            }

            ProximateShellError::UntaggedRuntimeError { reason } => Diagnostic::error().with_message(format!("Error: {}", reason)),
            ProximateShellError::Unimplemented { reason } => Diagnostic::error().with_message(format!("Unimplemented: {}", reason)),

        };

        let notes = self.notes.clone();
        d.with_notes(notes)
    }
