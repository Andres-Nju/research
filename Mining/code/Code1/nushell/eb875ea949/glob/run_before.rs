    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;
        let path = current_dir(engine_state, stack)?;
        let glob_pattern: Spanned<String> = call.req(engine_state, stack, 0)?;
        let depth = call.get_flag(engine_state, stack, "depth")?;

        if glob_pattern.item.is_empty() {
            return Err(ShellError::GenericError(
                "glob pattern must not be empty".to_string(),
                "".to_string(),
                Some(glob_pattern.span),
                Some("add characters to the glob pattern".to_string()),
                Vec::new(),
            ));
        }

        let folder_depth = if let Some(depth) = depth {
            depth
        } else {
            usize::MAX
        };

        let glob = match WaxGlob::new(&glob_pattern.item) {
            Ok(p) => p,
            Err(e) => {
                return Err(ShellError::GenericError(
                    "error with glob pattern".to_string(),
                    "".to_string(),
                    None,
                    Some(format!("{}", e)),
                    Vec::new(),
                ))
            }
        };

        #[allow(clippy::needless_collect)]
        let glob_results: Vec<Value> = glob
            .walk_with_behavior(
                path,
                WalkBehavior {
                    depth: folder_depth,
                    ..Default::default()
                },
            )
            .flatten()
            .map(|entry| Value::String {
                val: entry.into_path().to_string_lossy().to_string(),
                span,
            })
            .collect();

        Ok(glob_results
            .into_iter()
            .into_pipeline_data(engine_state.ctrlc.clone()))
    }
