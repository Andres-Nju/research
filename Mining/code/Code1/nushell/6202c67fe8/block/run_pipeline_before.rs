fn run_pipeline(
    commands: &Pipeline,
    ctx: &EvaluationContext,
    mut input: InputStream,
    external_redirection: ExternalRedirection,
) -> Result<OutputStream, ShellError> {
    let num_commands = commands.list.len();
    for (command_num, command) in commands.list.iter().enumerate() {
        input = match command {
            ClassifiedCommand::Dynamic(call) => {
                let mut args = vec![];
                if let Some(positional) = &call.positional {
                    for pos in positional {
                        let result = run_expression_block(pos, ctx)?.into_vec();
                        args.push(result);
                    }
                }

                let block = run_expression_block(&call.head, ctx)?.into_vec();

                if block.len() != 1 {
                    return Err(ShellError::labeled_error(
                        "Dynamic commands must start with a block",
                        "needs to be a block",
                        call.head.span,
                    ));
                }

                match &block[0].value {
                    UntaggedValue::Block(captured_block) => {
                        ctx.scope.enter_scope();
                        ctx.scope.add_vars(&captured_block.captured.entries);
                        for (param, value) in captured_block
                            .block
                            .params
                            .positional
                            .iter()
                            .zip(args.iter())
                        {
                            ctx.scope.add_var(param.0.name(), value[0].clone());
                        }
                        let result =
                            run_block(&captured_block.block, ctx, input, external_redirection);
                        ctx.scope.exit_scope();

                        let result = result?;
                        return Ok(result);
                    }
                    _ => {
                        return Err(ShellError::labeled_error("Dynamic commands must start with a block (or variable pointing to a block)", "needs to be a block", call.head.span));
                    }
                }
            }

            ClassifiedCommand::Expr(expr) => run_expression_block(&*expr, ctx)?,

            ClassifiedCommand::Error(err) => return Err(err.clone().into()),

            ClassifiedCommand::Internal(left) => {
                if command_num == (num_commands - 1) {
                    let mut left = left.clone();
                    left.args.external_redirection = external_redirection;
                    run_internal_command(&left, ctx, input)?
                } else {
                    run_internal_command(left, ctx, input)?
                }
            }
        };
    }

    Ok(input)
}
