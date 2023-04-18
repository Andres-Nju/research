fn rm(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<PipelineData, ShellError> {
    let trash = call.has_flag("trash");
    #[cfg(all(
        feature = "trash-support",
        not(target_os = "android"),
        not(target_os = "ios")
    ))]
    let permanent = call.has_flag("permanent");
    let recursive = call.has_flag("recursive");
    let force = call.has_flag("force");
    let verbose = call.has_flag("verbose");
    // let interactive = call.has_flag("interactive");

    let ctrlc = engine_state.ctrlc.clone();

    let targets: Vec<Spanned<String>> = call.rest(engine_state, stack, 0)?;
    let span = call.head;

    let config = engine_state.get_config();

    let rm_always_trash = config.rm_always_trash;

    #[cfg(any(
        not(feature = "trash-support"),
        target_os = "android",
        target_os = "ios"
    ))]
    {
        if rm_always_trash {
            return Err(ShellError::GenericError(
                "Cannot execute `rm`; the current configuration specifies \
                    `rm_always_trash = true`, but the current nu executable was not \
                    built with feature `trash_support` or trash is not supported on \
                    your platform."
                    .into(),
                "trash required to be true but not supported".into(),
                Some(span),
                None,
                Vec::new(),
            ));
        } else if trash {
            return Err(ShellError::GenericError(
                "Cannot execute `rm` with option `--trash`; feature `trash-support` not \
                    enabled or trash is not supported on your platform"
                    .into(),
                "this option is only available if nu is built with the `trash-support` feature"
                    .into(),
                Some(span),
                None,
                Vec::new(),
            ));
        }
    }

    if targets.is_empty() {
        return Err(ShellError::GenericError(
            "rm requires target paths".into(),
            "needs parameter".into(),
            Some(span),
            None,
            Vec::new(),
        ));
    }

    let path = current_dir(engine_state, stack)?;

    let mut all_targets: HashMap<PathBuf, Span> = HashMap::new();
    for target in targets {
        if path.to_string_lossy() == target.item
            || path.as_os_str().to_string_lossy().starts_with(&format!(
                "{}{}",
                target.item,
                std::path::MAIN_SEPARATOR
            ))
        {
            return Err(ShellError::GenericError(
                "Cannot remove any parent directory".into(),
                "cannot remove any parent directory".into(),
                Some(target.span),
                None,
                Vec::new(),
            ));
        }

        let path = path.join(&target.item);
        match nu_glob::glob_with(
            &path.to_string_lossy(),
            nu_glob::MatchOptions {
                require_literal_leading_dot: true,
                ..GLOB_PARAMS
            },
        ) {
            Ok(files) => {
                for file in files {
                    match file {
                        Ok(ref f) => {
                            // It is not appropriate to try and remove the
                            // current directory or its parent when using
                            // glob patterns.
                            let name = f.display().to_string();
                            if name.ends_with("/.") || name.ends_with("/..") {
                                continue;
                            }

                            all_targets.entry(f.clone()).or_insert_with(|| target.span);
                        }
                        Err(e) => {
                            return Err(ShellError::GenericError(
                                format!("Could not remove {:}", path.to_string_lossy()),
                                e.to_string(),
                                Some(target.span),
                                None,
                                Vec::new(),
                            ));
                        }
                    }
                }
            }
            Err(e) => {
                return Err(ShellError::GenericError(
                    e.to_string(),
                    e.to_string(),
                    Some(call.head),
                    None,
                    Vec::new(),
                ))
            }
        };
    }

    if all_targets.is_empty() && !force {
        return Err(ShellError::GenericError(
            "No valid paths".into(),
            "no valid paths".into(),
            Some(call.head),
            None,
            Vec::new(),
        ));
    }

    Ok(all_targets
        .into_iter()
        .map(move |(f, _)| {
            let is_empty = || match f.read_dir() {
                Ok(mut p) => p.next().is_none(),
                Err(_) => false,
            };

            if let Ok(metadata) = f.symlink_metadata() {
                #[cfg(unix)]
                let is_socket = metadata.file_type().is_socket();
                #[cfg(unix)]
                let is_fifo = metadata.file_type().is_fifo();

                #[cfg(not(unix))]
                let is_socket = false;
                #[cfg(not(unix))]
                let is_fifo = false;

                if metadata.is_file()
                    || metadata.file_type().is_symlink()
                    || recursive
                    || is_socket
                    || is_fifo
                    || is_empty()
                {
                    let result;
                    #[cfg(all(
                        feature = "trash-support",
                        not(target_os = "android"),
                        not(target_os = "ios")
                    ))]
                    {
                        use std::io::Error;
                        result = if trash || (rm_always_trash && !permanent) {
                            trash::delete(&f).map_err(|e: trash::Error| {
                                Error::new(ErrorKind::Other, format!("{:?}", e))
                            })
                        } else if metadata.is_file() || is_socket || is_fifo {
                            std::fs::remove_file(&f)
                        } else {
                            std::fs::remove_dir_all(&f)
                        };
                    }
                    #[cfg(any(
                        not(feature = "trash-support"),
                        target_os = "android",
                        target_os = "ios"
                    ))]
                    {
                        result = if metadata.is_file() || is_socket || is_fifo {
                            std::fs::remove_file(&f)
                        } else {
                            std::fs::remove_dir_all(&f)
                        };
                    }

                    if let Err(e) = result {
                        let msg = format!("Could not delete because: {:}\nTry '--trash' flag", e);
                        Value::Error {
                            error: ShellError::GenericError(
                                msg,
                                e.to_string(),
                                Some(span),
                                None,
                                Vec::new(),
                            ),
                        }
                    } else if verbose {
                        let val = format!("deleted {:}", f.to_string_lossy());
                        Value::String { val, span }
                    } else {
                        Value::Nothing { span }
                    }
                } else {
                    let msg = format!("Cannot remove {:}. try --recursive", f.to_string_lossy());
                    Value::Error {
                        error: ShellError::GenericError(
                            msg,
                            "cannot remove non-empty directory".into(),
                            Some(span),
                            None,
                            Vec::new(),
                        ),
                    }
                }
            } else {
                let msg = format!("no such file or directory: {:}", f.to_string_lossy());
                Value::Error {
                    error: ShellError::GenericError(
                        msg,
                        "no such file or directory".into(),
                        Some(span),
                        None,
                        Vec::new(),
                    ),
                }
            }
        })
        .filter(|x| !matches!(x.get_type(), Type::Nothing))
        .into_pipeline_data(ctrlc))
}
