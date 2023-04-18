fn parse_call(
    mut lite_cmd: LiteCommand,
    end_of_pipeline: bool,
    scope: &dyn ParserScope,
) -> (Option<ClassifiedCommand>, Option<ParseError>) {
    expand_aliases_in_call(&mut lite_cmd, scope);

    let mut error = None;
    if lite_cmd.parts.is_empty() {
        return (None, None);
    } else if lite_cmd.parts[0].item.starts_with('^') {
        let mut name = lite_cmd.parts[0]
            .clone()
            .map(|v| v.chars().skip(1).collect::<String>());

        name.span = Span::new(name.span.start() + 1, name.span.end());

        // TODO this is the same as the `else` branch below, only the name differs. Find a way
        //      to share this functionality.
        let mut args = vec![];

        let (name, err) = parse_arg(SyntaxShape::String, scope, &name);
        let name_span = name.span;
        if error.is_none() {
            error = err;
        }
        args.push(name);

        for lite_arg in &lite_cmd.parts[1..] {
            let (expr, err) = parse_external_arg(lite_arg, scope);
            if error.is_none() {
                error = err;
            }
            args.push(expr);
        }

        return (
            Some(ClassifiedCommand::Internal(InternalCommand {
                name: "run_external".to_string(),
                name_span,
                args: hir::Call {
                    head: Box::new(SpannedExpression {
                        expr: Expression::string("run_external".to_string()),
                        span: name_span,
                    }),
                    positional: Some(args),
                    named: None,
                    span: name_span,
                    external_redirection: if end_of_pipeline {
                        ExternalRedirection::None
                    } else {
                        ExternalRedirection::Stdout
                    },
                },
            })),
            error,
        );
    } else if lite_cmd.parts[0].item.starts_with('{') {
        return parse_value_call(lite_cmd, scope);
    } else if lite_cmd.parts[0].item.starts_with('$')
        || lite_cmd.parts[0].item.starts_with('\"')
        || lite_cmd.parts[0].item.starts_with('\'')
        || (lite_cmd.parts[0].item.starts_with('-')
            && parse_arg(SyntaxShape::Number, scope, &lite_cmd.parts[0])
                .1
                .is_none())
        || (lite_cmd.parts[0].item.starts_with('-')
            && parse_arg(SyntaxShape::Range, scope, &lite_cmd.parts[0])
                .1
                .is_none())
        || lite_cmd.parts[0].item.starts_with('0')
        || lite_cmd.parts[0].item.starts_with('1')
        || lite_cmd.parts[0].item.starts_with('2')
        || lite_cmd.parts[0].item.starts_with('3')
        || lite_cmd.parts[0].item.starts_with('4')
        || lite_cmd.parts[0].item.starts_with('5')
        || lite_cmd.parts[0].item.starts_with('6')
        || lite_cmd.parts[0].item.starts_with('7')
        || lite_cmd.parts[0].item.starts_with('8')
        || lite_cmd.parts[0].item.starts_with('9')
        || lite_cmd.parts[0].item.starts_with('[')
        || lite_cmd.parts[0].item.starts_with('(')
    {
        let (_, expr, err) = parse_math_expression(0, &lite_cmd.parts[..], scope, false);
        error = error.or(err);
        return (Some(ClassifiedCommand::Expr(Box::new(expr))), error);
    } else if lite_cmd.parts.len() > 1 {
        // Check if it's a sub-command
        if let Some(signature) = scope.get_signature(&format!(
            "{} {}",
            lite_cmd.parts[0].item, lite_cmd.parts[1].item
        )) {
            let (mut internal_command, err) =
                parse_internal_command(&lite_cmd, scope, &signature, 1);

            error = error.or(err);
            internal_command.args.external_redirection = if end_of_pipeline {
                ExternalRedirection::None
            } else {
                ExternalRedirection::Stdout
            };
            return (Some(ClassifiedCommand::Internal(internal_command)), error);
        }
    }
    // Check if it's an internal command
    if let Some(signature) = scope.get_signature(&lite_cmd.parts[0].item) {
        if lite_cmd.parts[0].item == "def" {
            let error = parse_definition(&lite_cmd, scope);
            if error.is_some() {
                return (
                    Some(ClassifiedCommand::Expr(Box::new(garbage(lite_cmd.span())))),
                    error,
                );
            }
        }
        let (mut internal_command, err) = parse_internal_command(&lite_cmd, scope, &signature, 0);

        if internal_command.name == "source" {
            if lite_cmd.parts.len() != 2 {
                return (
                    Some(ClassifiedCommand::Internal(internal_command)),
                    Some(ParseError::argument_error(
                        lite_cmd.parts[0].clone(),
                        ArgumentError::MissingMandatoryPositional("a path for sourcing".into()),
                    )),
                );
            }
            if lite_cmd.parts[1].item.starts_with('$') {
                return (
                    Some(ClassifiedCommand::Internal(internal_command)),
                    Some(ParseError::mismatch(
                        "a filepath constant",
                        lite_cmd.parts[1].clone(),
                    )),
                );
            }
            if let Ok(contents) =
                std::fs::read_to_string(expand_path(&lite_cmd.parts[1].item).into_owned())
            {
                let _ = parse(&contents, 0, scope);
            } else {
                return (
                    Some(ClassifiedCommand::Internal(internal_command)),
                    Some(ParseError::argument_error(
                        lite_cmd.parts[1].clone(),
                        ArgumentError::BadValue("can't load source file".into()),
                    )),
                );
            }
        } else if lite_cmd.parts[0].item == "alias" {
            let error = parse_alias(&lite_cmd, scope);
            if error.is_none() {
                return (Some(ClassifiedCommand::Internal(internal_command)), None);
            } else {
                return (Some(ClassifiedCommand::Internal(internal_command)), error);
            }
        }

        error = error.or(err);
        internal_command.args.external_redirection = if end_of_pipeline {
            ExternalRedirection::None
        } else {
            ExternalRedirection::Stdout
        };
        (Some(ClassifiedCommand::Internal(internal_command)), error)
    } else {
        parse_external_call(&lite_cmd, end_of_pipeline, scope)
    }
}
