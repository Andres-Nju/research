async fn process_line(
    readline: Result<String, ReadlineError>,
    ctx: &mut Context,
    redirect_stdin: bool,
    cli_mode: bool,
) -> LineResult {
    match &readline {
        Ok(line) if line.trim() == "" => LineResult::Success(line.clone()),

        Ok(line) => {
            let line = chomp_newline(line);

            let result = match nu_parser::lite_parse(&line, 0) {
                Err(err) => {
                    return LineResult::Error(line.to_string(), err.into());
                }

                Ok(val) => val,
            };

            debug!("=== Parsed ===");
            debug!("{:#?}", result);

            let mut classified_block = nu_parser::classify_block(&result, ctx.registry());

            debug!("{:#?}", classified_block);
            //println!("{:#?}", pipeline);

            if let Some(failure) = classified_block.failed {
                return LineResult::Error(line.to_string(), failure.into());
            }

            // There's a special case to check before we process the pipeline:
            // If we're giving a path by itself
            // ...and it's not a command in the path
            // ...and it doesn't have any arguments
            // ...and we're in the CLI
            // ...then change to this directory
            if cli_mode
                && classified_block.block.block.len() == 1
                && classified_block.block.block[0].list.len() == 1
            {
                if let ClassifiedCommand::Internal(InternalCommand {
                    ref name, ref args, ..
                }) = classified_block.block.block[0].list[0]
                {
                    let internal_name = name;
                    let name = args
                        .positional
                        .as_ref()
                        .and_then(|potionals| {
                            potionals.get(0).map(|e| {
                                if let Expression::Literal(Literal::String(ref s)) = e.expr {
                                    &s
                                } else {
                                    ""
                                }
                            })
                        })
                        .unwrap_or("");

                    if internal_name == "run_external"
                        && args
                            .positional
                            .as_ref()
                            .map(|ref v| v.len() == 1)
                            .unwrap_or(true)
                        && args
                            .named
                            .as_ref()
                            .map(NamedArguments::is_empty)
                            .unwrap_or(true)
                        && canonicalize(ctx.shell_manager.path(), name).is_ok()
                        && Path::new(&name).is_dir()
                        && which::which(&name).is_err()
                    {
                        // Here we work differently if we're in Windows because of the expected Windows behavior
                        #[cfg(windows)]
                        {
                            if name.ends_with(':') {
                                // This looks like a drive shortcut. We need to a) switch drives and b) go back to the previous directory we were viewing on that drive
                                // But first, we need to save where we are now
                                let current_path = ctx.shell_manager.path();

                                let split_path: Vec<_> = current_path.split(':').collect();
                                if split_path.len() > 1 {
                                    ctx.windows_drives_previous_cwd
                                        .lock()
                                        .insert(split_path[0].to_string(), current_path);
                                }

                                let name = name.to_uppercase();
                                let new_drive: Vec<_> = name.split(':').collect();

                                if let Some(val) =
                                    ctx.windows_drives_previous_cwd.lock().get(new_drive[0])
                                {
                                    ctx.shell_manager.set_path(val.to_string());
                                    return LineResult::Success(line.to_string());
                                } else {
                                    ctx.shell_manager
                                        .set_path(format!("{}\\", name.to_string()));
                                    return LineResult::Success(line.to_string());
                                }
                            } else {
                                ctx.shell_manager.set_path(name.to_string());
                                return LineResult::Success(line.to_string());
                            }
                        }
                        #[cfg(not(windows))]
                        {
                            ctx.shell_manager.set_path(name.to_string());
                            return LineResult::Success(line.to_string());
                        }
                    }
                }
            }

            let input_stream = if redirect_stdin {
                let file = futures::io::AllowStdIo::new(std::io::stdin());
                let stream = FramedRead::new(file, MaybeTextCodec).map(|line| {
                    if let Ok(line) = line {
                        match line {
                            StringOrBinary::String(s) => Ok(Value {
                                value: UntaggedValue::Primitive(Primitive::String(s)),
                                tag: Tag::unknown(),
                            }),
                            StringOrBinary::Binary(b) => Ok(Value {
                                value: UntaggedValue::Primitive(Primitive::Binary(
                                    b.into_iter().collect(),
                                )),
                                tag: Tag::unknown(),
                            }),
                        }
                    } else {
                        panic!("Internal error: could not read lines of text from stdin")
                    }
                });
                stream.to_input_stream()
            } else {
                InputStream::empty()
            };

            classified_block.block.expand_it_usage();

            trace!("{:#?}", classified_block);
            let env = ctx.get_env();
            match run_block(&classified_block.block, ctx, input_stream, &Scope::env(env)).await {
                Ok(input) => {
                    // Running a pipeline gives us back a stream that we can then
                    // work through. At the top level, we just want to pull on the
                    // values to compute them.
                    use futures::stream::TryStreamExt;

                    let context = RunnableContext {
                        input,
                        shell_manager: ctx.shell_manager.clone(),
                        host: ctx.host.clone(),
                        ctrl_c: ctx.ctrl_c.clone(),
                        registry: ctx.registry.clone(),
                        name: Tag::unknown(),
                    };

                    if let Ok(mut output_stream) = crate::commands::autoview::autoview(context) {
                        loop {
                            match output_stream.try_next().await {
                                Ok(Some(ReturnSuccess::Value(Value {
                                    value: UntaggedValue::Error(e),
                                    ..
                                }))) => return LineResult::Error(line.to_string(), e),
                                Ok(Some(_item)) => {
                                    if ctx.ctrl_c.load(Ordering::SeqCst) {
                                        break;
                                    }
                                }
                                Ok(None) => break,
                                Err(e) => return LineResult::Error(line.to_string(), e),
                            }
                        }
                    }

                    LineResult::Success(line.to_string())
                }
                Err(err) => LineResult::Error(line.to_string(), err),
            }
        }
        Err(ReadlineError::Interrupted) => LineResult::CtrlC,
        Err(ReadlineError::Eof) => LineResult::Break,
        Err(err) => {
            outln!("Error: {:?}", err);
            LineResult::Break
        }
    }
}
