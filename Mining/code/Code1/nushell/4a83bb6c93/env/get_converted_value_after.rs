fn get_converted_value(
    engine_state: &EngineState,
    stack: &Stack,
    name: &str,
    orig_val: &Value,
    direction: &str,
) -> ConversionResult {
    if let Some(env_conversions) = stack.get_env_var(engine_state, ENV_CONVERSIONS) {
        let env_span = match env_conversions.span() {
            Ok(span) => span,
            Err(e) => {
                return ConversionResult::GeneralError(e);
            }
        };
        let val_span = match orig_val.span() {
            Ok(span) => span,
            Err(e) => {
                return ConversionResult::GeneralError(e);
            }
        };

        let path_members = &[
            PathMember::String {
                val: name.to_string(),
                span: env_span,
            },
            PathMember::String {
                val: direction.to_string(),
                span: env_span,
            },
        ];

        if let Ok(Value::Closure {
            val: block_id,
            span: from_span,
            ..
        }) = env_conversions.follow_cell_path_not_from_user_input(path_members, false)
        {
            let block = engine_state.get_block(block_id);

            if let Some(var) = block.signature.get_positional(0) {
                let mut stack = stack.gather_captures(&block.captures);
                if let Some(var_id) = &var.var_id {
                    stack.add_var(*var_id, orig_val.clone());
                }

                let result = eval_block(
                    engine_state,
                    &mut stack,
                    block,
                    PipelineData::new(val_span),
                    true,
                    true,
                );

                match result {
                    Ok(data) => ConversionResult::Ok(data.into_value(val_span)),
                    Err(e) => ConversionResult::ConversionError(e),
                }
            } else {
                ConversionResult::ConversionError(ShellError::MissingParameter(
                    "block input".into(),
                    from_span,
                ))
            }
        } else {
            ConversionResult::CellPathError
        }
    } else {
        ConversionResult::CellPathError
    }
}
