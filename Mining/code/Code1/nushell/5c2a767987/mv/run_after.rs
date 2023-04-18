    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // TODO: handle invalid directory or insufficient permissions when moving
        let spanned_source: Spanned<String> = call.req(engine_state, stack, 0)?;
        let spanned_source = {
            Spanned {
                item: nu_utils::strip_ansi_string_unlikely(spanned_source.item),
                span: spanned_source.span,
            }
        };
        let spanned_destination: Spanned<String> = call.req(engine_state, stack, 1)?;
        let verbose = call.has_flag("verbose");
        let interactive = call.has_flag("interactive");
        let force = call.has_flag("force");

        let ctrlc = engine_state.ctrlc.clone();

        let path = current_dir(engine_state, stack)?;
        let source = path.join(spanned_source.item.as_str());
        let destination = path.join(spanned_destination.item.as_str());

        let mut sources = nu_glob::glob_with(&source.to_string_lossy(), GLOB_PARAMS)
            .map_or_else(|_| Vec::new(), Iterator::collect);

        if sources.is_empty() {
            return Err(ShellError::GenericError(
                "File(s) not found".into(),
                "could not find any files matching this glob pattern".into(),
                Some(spanned_source.span),
                None,
                Vec::new(),
            ));
        }

        // We have two possibilities.
        //
        // First, the destination exists.
        //  - If a directory, move everything into that directory, otherwise
        //  - if only a single source, and --force (or -f) is provided overwrite the file,
        //  - otherwise error.
        //
        // Second, the destination doesn't exist, so we can only rename a single source. Otherwise
        // it's an error.

        if destination.exists() && !force && !destination.is_dir() && !source.is_dir() {
            return Err(ShellError::GenericError(
                "Destination file already exists".into(),
                // These messages all use to_string_lossy() because
                // showing the full path reduces misinterpretation of the message.
                // Also, this is preferable to {:?} because that renders Windows paths incorrectly.
                format!(
                    "Destination file '{}' already exists",
                    destination.to_string_lossy()
                ),
                Some(spanned_destination.span),
                Some("you can use -f, --force to force overwriting the destination".into()),
                Vec::new(),
            ));
        }

        if (destination.exists() && !destination.is_dir() && sources.len() > 1)
            || (!destination.exists() && sources.len() > 1)
        {
            return Err(ShellError::GenericError(
                "Can only move multiple sources if destination is a directory".into(),
                "destination must be a directory when moving multiple sources".into(),
                Some(spanned_destination.span),
                None,
                Vec::new(),
            ));
        }

        // This is the case where you move a directory A to the interior of directory B, but directory B
        // already has a non-empty directory named A.
        if source.is_dir() && destination.is_dir() {
            if let Some(name) = source.file_name() {
                let dst = destination.join(name);
                if dst.is_dir() {
                    return Err(ShellError::GenericError(
                        format!(
                            "Can't move '{}' to '{}'",
                            source.to_string_lossy(),
                            dst.to_string_lossy()
                        ),
                        format!("Directory '{}' is not empty", destination.to_string_lossy()),
                        Some(spanned_destination.span),
                        None,
                        Vec::new(),
                    ));
                }
            }
        }

        let some_if_source_is_destination = sources
            .iter()
            .find(|f| matches!(f, Ok(f) if destination.starts_with(f)));
        if destination.exists() && destination.is_dir() && sources.len() == 1 {
            if let Some(Ok(filename)) = some_if_source_is_destination {
                return Err(ShellError::GenericError(
                    format!(
                        "Not possible to move '{}' to itself",
                        filename.to_string_lossy()
                    ),
                    "cannot move to itself".into(),
                    Some(spanned_destination.span),
                    None,
                    Vec::new(),
                ));
            }
        }

        if let Some(Ok(_filename)) = some_if_source_is_destination {
            sources.retain(|f| matches!(f, Ok(f) if !destination.starts_with(f)));
        }

        let span = call.head;
        sources
            .into_iter()
            .flatten()
            .filter_map(move |entry| {
                let result = move_file(
                    Spanned {
                        item: entry.clone(),
                        span: spanned_source.span,
                    },
                    Spanned {
                        item: destination.clone(),
                        span: spanned_destination.span,
                    },
                    interactive,
                );
                if let Err(error) = result {
                    Some(Value::Error {
                        error: Box::new(error),
                    })
                } else if verbose {
                    let val = match result {
                        Ok(true) => format!(
                            "moved {:} to {:}",
                            entry.to_string_lossy(),
                            destination.to_string_lossy()
                        ),
                        _ => format!(
                            "{:} not moved to {:}",
                            entry.to_string_lossy(),
                            destination.to_string_lossy()
                        ),
                    };
                    Some(Value::String { val, span })
                } else {
                    None
                }
            })
            .into_pipeline_data(ctrlc)
            .print_not_formatted(engine_state, false, true)?;
        Ok(PipelineData::empty())
    }
