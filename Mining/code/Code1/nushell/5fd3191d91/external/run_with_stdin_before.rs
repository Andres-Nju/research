async fn run_with_stdin(
    command: ExternalCommand,
    context: &mut Context,
    input: Option<InputStream>,
    is_last: bool,
) -> Result<Option<InputStream>, ShellError> {
    let name_tag = command.name_tag;
    let home_dir = dirs::home_dir();

    let mut process = Exec::cmd(&command.name);
    for arg in command.args.iter() {
        // Let's also replace ~ as we shell out
        let arg = shellexpand::tilde_with_context(arg.deref(), || home_dir.as_ref());

        // Strip quotes from a quoted string
        if arg.len() > 1
            && ((arg.starts_with('"') && arg.ends_with('"'))
                || (arg.starts_with('\'') && arg.ends_with('\'')))
        {
            process = process.arg(arg.chars().skip(1).take(arg.len() - 2).collect::<String>());
        } else {
            process = process.arg(arg.as_ref());
        }
    }

    process = process.cwd(context.shell_manager.path()?);
    trace!(target: "nu::run::external", "cwd = {:?}", context.shell_manager.path());

    if !is_last {
        process = process.stdout(subprocess::Redirection::Pipe);
        trace!(target: "nu::run::external", "set up stdout pipe");
    }

    if input.is_some() {
        process = process.stdin(subprocess::Redirection::Pipe);
        trace!(target: "nu::run::external", "set up stdin pipe");
    }

    trace!(target: "nu::run::external", "built process {:?}", process);

    let popen = process.detached().popen();
    if let Ok(mut popen) = popen {
        let stream = async_stream! {
            if let Some(mut input) = input {
                let mut stdin_write = popen
                    .stdin
                    .take()
                    .expect("Internal error: could not get stdin pipe for external command");

                while let Some(item) = input.next().await {
                    match item.value {
                        UntaggedValue::Primitive(Primitive::Nothing) => {
                            // If first in a pipeline, will receive Nothing. This is not an error.
                        },

                        UntaggedValue::Primitive(Primitive::String(s)) |
                            UntaggedValue::Primitive(Primitive::Line(s)) =>
                        {
                            if let Err(e) = stdin_write.write(s.as_bytes()) {
                                let message = format!("Unable to write to stdin (error = {})", e);
                                yield Ok(Value {
                                    value: UntaggedValue::Error(ShellError::labeled_error(
                                        message,
                                        "unable to write to stdin",
                                        &name_tag,
                                    )),
                                    tag: name_tag,
                                });
                                return;
                            }
                        },

                        // TODO serialize other primitives? https://github.com/nushell/nushell/issues/778

                        v => {
                            let message = format!("Received unexpected type from pipeline ({})", v.type_name());
                            yield Ok(Value {
                                value: UntaggedValue::Error(ShellError::labeled_error(
                                    message,
                                    "expected a string",
                                    &name_tag,
                                )),
                                tag: name_tag,
                            });
                            return;
                        },
                    }
                }

                // Close stdin, which informs the external process that there's no more input
                drop(stdin_write);
            }

            if !is_last {
                let stdout = if let Some(stdout) = popen.stdout.take() {
                    stdout
                } else {
                    yield Ok(Value {
                        value: UntaggedValue::Error(
                            ShellError::labeled_error(
                                "Can't redirect the stdout for external command",
                                "can't redirect stdout",
                                &name_tag,
                            )
                        ),
                        tag: name_tag,
                    });
                    return;
                };

                let file = futures::io::AllowStdIo::new(stdout);
                let stream = Framed::new(file, LinesCodec {});
                let mut stream = stream.map(|line| {
                    if let Ok(line) = line {
                        line.into_value(&name_tag)
                    } else {
                        panic!("Internal error: could not read lines of text from stdin")
                    }
                });

                loop {
                    match stream.next().await {
                        Some(item) => yield Ok(item),
                        None => break,
                    }
                }
            }

            let errored = match popen.wait() {
                Ok(status) => !status.success(),
                Err(e) => true,
            };

            if errored {
                yield Ok(Value {
                    value: UntaggedValue::Error(
                        ShellError::labeled_error(
                            "External command failed",
                            "command failed",
                            &name_tag,
                        )
                    ),
                    tag: name_tag,
                });
            };
        };

        Ok(Some(stream.to_input_stream()))
    } else {
        Err(ShellError::labeled_error(
            "Command not found",
            "command not found",
            name_tag,
        ))
    }
}
