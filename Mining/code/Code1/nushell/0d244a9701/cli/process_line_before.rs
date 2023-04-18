async fn process_line(
    readline: Result<String, ReadlineError>,
    ctx: &mut Context,
    redirect_stdin: bool,
) -> LineResult {
    match &readline {
        Ok(line) if line.trim() == "" => LineResult::Success(line.clone()),

        Ok(line) => {
            let line = chomp_newline(line);

            let result = match nu_parser::parse(&line) {
                Err(err) => {
                    return LineResult::Error(line.to_string(), err);
                }

                Ok(val) => val,
            };

            debug!("=== Parsed ===");
            debug!("{:#?}", result);

            let pipeline = classify_pipeline(&result, ctx, &Text::from(line));

            if let Some(failure) = pipeline.failed {
                return LineResult::Error(line.to_string(), failure.into());
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
                Some(stream.to_input_stream())
            } else {
                None
            };

            match run_pipeline(pipeline, ctx, input_stream, line).await {
                Ok(Some(input)) => {
                    // Running a pipeline gives us back a stream that we can then
                    // work through. At the top level, we just want to pull on the
                    // values to compute them.
                    use futures::stream::TryStreamExt;

                    let context = RunnableContext {
                        input,
                        shell_manager: ctx.shell_manager.clone(),
                        host: ctx.host.clone(),
                        ctrl_c: ctx.ctrl_c.clone(),
                        commands: ctx.registry.clone(),
                        name: Tag::unknown(),
                        source: Text::from(String::new()),
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
                                _ => {
                                    break;
                                }
                            }
                        }
                    }

                    LineResult::Success(line.to_string())
                }
                Ok(None) => LineResult::Success(line.to_string()),
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
