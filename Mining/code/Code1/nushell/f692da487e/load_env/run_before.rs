    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let arg: Option<(Vec<String>, Vec<Value>)> = call.opt(engine_state, stack, 0)?;
        let span = call.head;

        match arg {
            Some((cols, vals)) => {
                for (env_var, rhs) in cols.into_iter().zip(vals) {
                    if env_var == "PWD" {
                        let cwd = current_dir(engine_state, stack)?;
                        let rhs = rhs.as_string()?;
                        let rhs = nu_path::expand_path_with(rhs, cwd);
                        stack.add_env_var(
                            env_var,
                            Value::String {
                                val: rhs.to_string_lossy().to_string(),
                                span: call.head,
                            },
                        );
                    } else {
                        stack.add_env_var(env_var, rhs);
                    }
                }
                Ok(PipelineData::new(call.head))
            }
            None => match input {
                PipelineData::Value(Value::Record { cols, vals, .. }, ..) => {
                    for (env_var, rhs) in cols.into_iter().zip(vals) {
                        if env_var == "PWD" {
                            let cwd = current_dir(engine_state, stack)?;
                            let rhs = rhs.as_string()?;
                            let rhs = nu_path::expand_path_with(rhs, cwd);
                            stack.add_env_var(
                                env_var,
                                Value::String {
                                    val: rhs.to_string_lossy().to_string(),
                                    span: call.head,
                                },
                            );
                        } else {
                            stack.add_env_var(env_var, rhs);
                        }
                    }
                    Ok(PipelineData::new(call.head))
                }
                _ => Err(ShellError::UnsupportedInput(
                    "Record not supported".into(),
                    span,
                )),
            },
        }
    }
