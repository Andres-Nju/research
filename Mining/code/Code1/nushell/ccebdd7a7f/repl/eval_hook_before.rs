pub fn eval_hook(
    engine_state: &mut EngineState,
    stack: &mut Stack,
    arguments: Vec<(String, Value)>,
    value: &Value,
) -> Result<(), ShellError> {
    let value_span = value.span()?;

    let condition_path = PathMember::String {
        val: "condition".to_string(),
        span: value_span,
    };

    let code_path = PathMember::String {
        val: "code".to_string(),
        span: value_span,
    };

    match value {
        Value::List { vals, .. } => {
            for val in vals {
                eval_hook(engine_state, stack, arguments.clone(), val)?
            }
        }
        Value::Record { .. } => {
            let do_run_hook =
                if let Ok(condition) = value.clone().follow_cell_path(&[condition_path], false) {
                    match condition {
                        Value::Block {
                            val: block_id,
                            span: block_span,
                            ..
                        } => {
                            match run_hook_block(
                                engine_state,
                                stack,
                                block_id,
                                arguments.clone(),
                                block_span,
                            ) {
                                Ok(value) => match value {
                                    Value::Bool { val, .. } => val,
                                    other => {
                                        return Err(ShellError::UnsupportedConfigValue(
                                            "boolean output".to_string(),
                                            format!("{}", other.get_type()),
                                            other.span()?,
                                        ));
                                    }
                                },
                                Err(err) => {
                                    return Err(err);
                                }
                            }
                        }
                        other => {
                            return Err(ShellError::UnsupportedConfigValue(
                                "block".to_string(),
                                format!("{}", other.get_type()),
                                other.span()?,
                            ));
                        }
                    }
                } else {
                    // always run the hook
                    true
                };

            if do_run_hook {
                match value.clone().follow_cell_path(&[code_path], false)? {
                    Value::String {
                        val,
                        span: source_span,
                    } => {
                        let (block, delta, vars) = {
                            let mut working_set = StateWorkingSet::new(engine_state);

                            let mut vars: Vec<(VarId, Value)> = vec![];

                            for (name, val) in arguments {
                                let var_id = working_set.add_variable(
                                    name.as_bytes().to_vec(),
                                    val.span()?,
                                    Type::Any,
                                );

                                vars.push((var_id, val));
                            }

                            let (output, err) =
                                parse(&mut working_set, Some("hook"), val.as_bytes(), false, &[]);
                            if let Some(err) = err {
                                report_error(&working_set, &err);

                                return Err(ShellError::UnsupportedConfigValue(
                                    "valid source code".into(),
                                    "source code with syntax errors".into(),
                                    source_span,
                                ));
                            }

                            (output, working_set.render(), vars)
                        };

                        engine_state.merge_delta(delta)?;
                        let input = PipelineData::new(value_span);

                        let var_ids: Vec<VarId> = vars
                            .into_iter()
                            .map(|(var_id, val)| {
                                stack.add_var(var_id, val);
                                var_id
                            })
                            .collect();

                        match eval_block(engine_state, stack, &block, input, false, false) {
                            Ok(_) => {}
                            Err(err) => {
                                report_error_new(engine_state, &err);
                            }
                        }

                        for var_id in var_ids.iter() {
                            stack.vars.remove(var_id);
                        }

                        let cwd = get_guaranteed_cwd(engine_state, stack);
                        engine_state.merge_env(stack, cwd)?;
                    }
                    Value::Block {
                        val: block_id,
                        span: block_span,
                        ..
                    } => {
                        run_hook_block(engine_state, stack, block_id, arguments, block_span)?;
                        let cwd = get_guaranteed_cwd(engine_state, stack);
                        engine_state.merge_env(stack, cwd)?;
                    }
                    other => {
                        return Err(ShellError::UnsupportedConfigValue(
                            "block or string".to_string(),
                            format!("{}", other.get_type()),
                            other.span()?,
                        ));
                    }
                }
            }
        }
        Value::Block {
            val: block_id,
            span: block_span,
            ..
        } => {
            run_hook_block(engine_state, stack, *block_id, arguments, *block_span)?;
        }
        other => {
            return Err(ShellError::UnsupportedConfigValue(
                "block, record, or list of records".into(),
                format!("{}", other.get_type()),
                other.span()?,
            ));
        }
    }

    Ok(())
}
