fn classify_pipeline(
    lite_pipeline: &LitePipeline,
    registry: &dyn SignatureRegistry,
) -> (ClassifiedPipeline, Option<ParseError>) {
    // FIXME: fake span
    let mut commands = Commands::new(Span::new(0, 0));
    let mut error = None;

    let mut iter = lite_pipeline.commands.iter().peekable();
    while let Some(lite_cmd) = iter.next() {
        if lite_cmd.name.item.starts_with('^') {
            let name = lite_cmd
                .name
                .clone()
                .map(|v| v.chars().skip(1).collect::<String>());
            // TODO this is the same as the `else` branch below, only the name differs. Find a way
            //      to share this functionality.
            let mut args = vec![];

            let (name, err) = parse_arg(SyntaxShape::String, registry, &name);
            let name_span = name.span;
            if error.is_none() {
                error = err;
            }
            args.push(name);

            for lite_arg in &lite_cmd.args {
                let (expr, err) = parse_external_arg(registry, lite_arg);
                if error.is_none() {
                    error = err;
                }
                args.push(expr);
            }

            commands.push(ClassifiedCommand::Internal(InternalCommand {
                name: "run_external".to_string(),
                name_span,
                args: hir::Call {
                    head: Box::new(SpannedExpression {
                        expr: Expression::string("run_external".to_string()),
                        span: name_span,
                    }),
                    positional: Some(args),
                    named: None,
                    span: Span::unknown(),
                    is_last: iter.peek().is_none(),
                },
            }))
        } else if lite_cmd.name.item == "=" {
            let expr = if !lite_cmd.args.is_empty() {
                let (_, expr, err) = parse_math_expression(0, &lite_cmd.args[0..], registry, false);
                error = error.or(err);
                expr
            } else {
                error = error.or_else(|| {
                    Some(ParseError::argument_error(
                        lite_cmd.name.clone(),
                        ArgumentError::MissingMandatoryPositional("an expression".into()),
                    ))
                });
                garbage(lite_cmd.span())
            };
            commands.push(ClassifiedCommand::Expr(Box::new(expr)))
        } else {
            if !lite_cmd.args.is_empty() {
                // Check if it's a sub-command
                if let Some(signature) =
                    registry.get(&format!("{} {}", lite_cmd.name.item, lite_cmd.args[0].item))
                {
                    let (internal_command, err) =
                        parse_internal_command(&lite_cmd, registry, &signature, 1);

                    error = error.or(err);
                    commands.push(ClassifiedCommand::Internal(internal_command));
                    continue;
                }
            }

            // Check if it's an internal command
            if let Some(signature) = registry.get(&lite_cmd.name.item) {
                let (internal_command, err) =
                    parse_internal_command(&lite_cmd, registry, &signature, 0);

                error = error.or(err);
                commands.push(ClassifiedCommand::Internal(internal_command));
                continue;
            }

            let name = lite_cmd.name.clone().map(|v| {
                let trimmed = trim_quotes(&v);
                expand_path(&trimmed).to_string()
            });

            let mut args = vec![];

            let (name, err) = parse_arg(SyntaxShape::String, registry, &name);
            let name_span = name.span;
            if error.is_none() {
                error = err;
            }
            args.push(name);

            for lite_arg in &lite_cmd.args {
                let (expr, err) = parse_external_arg(registry, lite_arg);
                if error.is_none() {
                    error = err;
                }
                args.push(expr);
            }

            commands.push(ClassifiedCommand::Internal(InternalCommand {
                name: "run_external".to_string(),
                name_span,
                args: hir::Call {
                    head: Box::new(SpannedExpression {
                        expr: Expression::string("run_external".to_string()),
                        span: name_span,
                    }),
                    positional: Some(args),
                    named: None,
                    span: Span::unknown(),
                    is_last: iter.peek().is_none(),
                },
            }))
        }
    }

    (ClassifiedPipeline::new(commands), error)
}
