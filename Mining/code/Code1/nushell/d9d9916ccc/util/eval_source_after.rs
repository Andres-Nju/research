pub fn eval_source(
    engine_state: &mut EngineState,
    stack: &mut Stack,
    source: &[u8],
    fname: &str,
    input: PipelineData,
) -> bool {
    let (block, delta) = {
        let mut working_set = StateWorkingSet::new(engine_state);
        let (output, err) = parse(
            &mut working_set,
            Some(fname), // format!("entry #{}", entry_num)
            source,
            false,
            &[],
        );
        if let Some(err) = err {
            set_last_exit_code(stack, 1);
            report_error(&working_set, &err);
            return false;
        }

        (output, working_set.render())
    };

    if let Err(err) = engine_state.merge_delta(delta) {
        set_last_exit_code(stack, 1);
        report_error_new(engine_state, &err);
        return false;
    }

    match eval_block(engine_state, stack, &block, input, false, false) {
        Ok(pipeline_data) => {
            let config = engine_state.get_config();
            let result;
            if let PipelineData::ExternalStream {
                stdout: stream,
                stderr: stderr_stream,
                exit_code,
                ..
            } = pipeline_data
            {
                result = print_if_stream(stream, stderr_stream, false, exit_code);
            } else if let Some(hook) = config.hooks.display_output.clone() {
                match eval_hook(engine_state, stack, Some(pipeline_data), vec![], &hook) {
                    Err(err) => {
                        result = Err(err);
                    }
                    Ok(val) => {
                        result = val.print(engine_state, stack, false, false);
                    }
                }
            } else {
                result = pipeline_data.print(engine_state, stack, true, false);
            }

            match result {
                Err(err) => {
                    let working_set = StateWorkingSet::new(engine_state);

                    report_error(&working_set, &err);

                    return false;
                }
                Ok(exit_code) => {
                    set_last_exit_code(stack, exit_code);
                }
            }

            // reset vt processing, aka ansi because illbehaved externals can break it
            #[cfg(windows)]
            {
                let _ = enable_vt_processing();
            }
        }
        Err(err) => {
            set_last_exit_code(stack, 1);

            let working_set = StateWorkingSet::new(engine_state);

            report_error(&working_set, &err);

            return false;
        }
    }

    true
}
