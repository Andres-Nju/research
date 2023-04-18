    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let cond = call.positional_nth(0).expect("checked through parser");
        let block: Block = call.req(engine_state, stack, 1)?;

        loop {
            if let Some(ctrlc) = &engine_state.ctrlc {
                if ctrlc.load(Ordering::SeqCst) {
                    break;
                }
            }

            let result = eval_expression(engine_state, stack, cond)?;
            match &result {
                Value::Bool { val, .. } => {
                    if *val {
                        let block = engine_state.get_block(block.block_id);
                        eval_block(
                            engine_state,
                            stack,
                            block,
                            PipelineData::new(call.head),
                            call.redirect_stdout,
                            call.redirect_stderr,
                        )?
                        .into_value(call.head);
                    } else {
                        break;
                    }
                }
                x => {
                    return Err(ShellError::CantConvert(
                        "bool".into(),
                        x.get_type().to_string(),
                        result.span()?,
                        None,
                    ))
                }
            }
        }
        Ok(PipelineData::new(call.head))
    }
