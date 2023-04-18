    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let block: Closure = call.req(engine_state, stack, 0)?;
        let rest: Vec<Value> = call.rest(engine_state, stack, 1)?;
        let ignore_all_errors = call.has_flag("ignore-errors");
        let ignore_shell_errors = ignore_all_errors || call.has_flag("ignore-shell-errors");
        let ignore_program_errors = ignore_all_errors || call.has_flag("ignore-program-errors");
        let capture_errors = call.has_flag("capture-errors");

        let mut stack = stack.captures_to_stack(&block.captures);
        let block = engine_state.get_block(block.block_id);

        let params: Vec<_> = block
            .signature
            .required_positional
            .iter()
            .chain(block.signature.optional_positional.iter())
            .collect();

        for param in params.iter().zip(&rest) {
            if let Some(var_id) = param.0.var_id {
                stack.add_var(var_id, param.1.clone())
            }
        }

        if let Some(param) = &block.signature.rest_positional {
            if rest.len() > params.len() {
                let mut rest_items = vec![];

                for r in rest.into_iter().skip(params.len()) {
                    rest_items.push(r);
                }

                let span = if let Some(rest_item) = rest_items.first() {
                    rest_item.span()?
                } else {
                    call.head
                };

                stack.add_var(
                    param
                        .var_id
                        .expect("Internal error: rest positional parameter lacks var_id"),
                    Value::List {
                        vals: rest_items,
                        span,
                    },
                )
            }
        }
        let result = eval_block_with_early_return(
            engine_state,
            &mut stack,
            block,
            input,
            call.redirect_stdout,
            call.redirect_stdout,
        );

        match result {
            Ok(PipelineData::ExternalStream {
                stdout,
                stderr,
                exit_code,
                span,
                metadata,
                trim_end_newline,
            }) if capture_errors => {
                // Use a thread to receive stdout message.
                // Or we may get a deadlock if child process sends out too much bytes to stderr.
                //
                // For example: in normal linux system, stderr pipe's limit is 65535 bytes.
                // if child process sends out 65536 bytes, the process will be hanged because no consumer
                // consumes the first 65535 bytes
                // So we need a thread to receive stdout message, then the current thread can continue to consume
                // stderr messages.
                let stdout_handler = stdout.map(|stdout_stream| {
                    thread::spawn(move || {
                        let ctrlc = stdout_stream.ctrlc.clone();
                        let span = stdout_stream.span;
                        RawStream::new(
                            Box::new(vec![stdout_stream.into_bytes().map(|s| s.item)].into_iter()),
                            ctrlc,
                            span,
                            None,
                        )
                    })
                });

                // Intercept stderr so we can return it in the error if the exit code is non-zero.
                // The threading issues mentioned above dictate why we also need to intercept stdout.
                let mut stderr_ctrlc = None;
                let stderr_msg = match stderr {
                    None => "".to_string(),
                    Some(stderr_stream) => {
                        stderr_ctrlc = stderr_stream.ctrlc.clone();
                        stderr_stream.into_string().map(|s| s.item)?
                    }
                };

                let stdout = if let Some(handle) = stdout_handler {
                    match handle.join() {
                        Err(err) => {
                            return Err(ShellError::ExternalCommand(
                                "Fail to receive external commands stdout message".to_string(),
                                format!("{err:?}"),
                                span,
                            ));
                        }
                        Ok(res) => Some(res),
                    }
                } else {
                    None
                };

                let mut exit_code_ctrlc = None;
                let exit_code: Vec<Value> = match exit_code {
                    None => vec![],
                    Some(exit_code_stream) => {
                        exit_code_ctrlc = exit_code_stream.ctrlc.clone();
                        exit_code_stream.into_iter().collect()
                    }
                };
                if let Some(Value::Int { val: code, .. }) = exit_code.last() {
                    if *code != 0 {
                        return Err(ShellError::ExternalCommand(
                            "External command failed".to_string(),
                            stderr_msg,
                            span,
                        ));
                    }
                }

                Ok(PipelineData::ExternalStream {
                    stdout,
                    stderr: Some(RawStream::new(
                        Box::new(vec![Ok(stderr_msg.into_bytes())].into_iter()),
                        stderr_ctrlc,
                        span,
                        None,
                    )),
                    exit_code: Some(ListStream::from_stream(
                        exit_code.into_iter(),
                        exit_code_ctrlc,
                    )),
                    span,
                    metadata,
                    trim_end_newline,
                })
            }
            Ok(PipelineData::ExternalStream {
                stdout,
                stderr,
                exit_code: _,
                span,
                metadata,
                trim_end_newline,
            }) if ignore_program_errors => Ok(PipelineData::ExternalStream {
                stdout,
                stderr,
                exit_code: None,
                span,
                metadata,
                trim_end_newline,
            }),
            Ok(PipelineData::Value(..)) | Err(_) if ignore_shell_errors => {
                Ok(PipelineData::empty())
            }
            r => r,
        }
    }
