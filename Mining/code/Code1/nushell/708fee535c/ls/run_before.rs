    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let all = call.has_flag("all");
        let long = call.has_flag("long");
        let short_names = call.has_flag("short-names");
        let full_paths = call.has_flag("full-paths");
        let du = call.has_flag("du");
        let directory = call.has_flag("directory");
        let ctrl_c = engine_state.ctrlc.clone();
        let call_span = call.head;
        let cwd = current_dir(engine_state, stack)?;

        let pattern_arg: Option<Spanned<String>> = call.opt(engine_state, stack, 0)?;

        let pattern_arg = {
            if let Some(path) = pattern_arg {
                Some(Spanned {
                    item: nu_utils::strip_ansi_string_unlikely(path.item),
                    span: path.span,
                })
            } else {
                pattern_arg
            }
        };

        let (path, p_tag, absolute_path) = match pattern_arg {
            Some(p) => {
                let p_tag = p.span;
                let mut p = expand_to_real_path(p.item);

                let expanded = nu_path::expand_path_with(&p, &cwd);
                // Avoid checking and pushing "*" to the path when directory (do not show contents) flag is true
                if !directory && expanded.is_dir() {
                    if permission_denied(&p) {
                        #[cfg(unix)]
                        let error_msg = format!(
                            "The permissions of {:o} do not allow access for this user",
                            expanded
                                .metadata()
                                .expect(
                                    "this shouldn't be called since we already know there is a dir"
                                )
                                .permissions()
                                .mode()
                                & 0o0777
                        );
                        #[cfg(not(unix))]
                        let error_msg = String::from("Permission denied");
                        return Err(ShellError::GenericError(
                            "Permission denied".to_string(),
                            error_msg,
                            Some(p_tag),
                            None,
                            Vec::new(),
                        ));
                    }
                    if is_empty_dir(&expanded) {
                        return Ok(Value::nothing(call_span).into_pipeline_data());
                    }
                    p.push("*");
                }
                let absolute_path = p.is_absolute();
                (p, p_tag, absolute_path)
            }
            None => {
                // Avoid pushing "*" to the default path when directory (do not show contents) flag is true
                if directory {
                    (PathBuf::from("."), call_span, false)
                } else if is_empty_dir(current_dir(engine_state, stack)?) {
                    return Ok(Value::nothing(call_span).into_pipeline_data());
                } else {
                    (PathBuf::from("./*"), call_span, false)
                }
            }
        };

        let hidden_dir_specified = is_hidden_dir(&path);

        let glob_path = Spanned {
            item: path.display().to_string(),
            span: p_tag,
        };

        let glob_options = if all {
            None
        } else {
            let mut glob_options = MatchOptions::new();
            glob_options.recursive_match_hidden_dir = false;
            Some(glob_options)
        };
        let (prefix, paths) = nu_engine::glob_from(&glob_path, &cwd, call_span, glob_options)?;

        let mut paths_peek = paths.peekable();
        if paths_peek.peek().is_none() {
            return Err(ShellError::GenericError(
                format!("No matches found for {}", &path.display().to_string()),
                "".to_string(),
                None,
                Some("no matches found".to_string()),
                Vec::new(),
            ));
        }

        let mut hidden_dirs = vec![];

        Ok(paths_peek
            .into_iter()
            .filter_map(move |x| match x {
                Ok(path) => {
                    let metadata = match std::fs::symlink_metadata(&path) {
                        Ok(metadata) => Some(metadata),
                        Err(_) => None,
                    };
                    if path_contains_hidden_folder(&path, &hidden_dirs) {
                        return None;
                    }

                    if !all && !hidden_dir_specified && is_hidden_dir(&path) {
                        if path.is_dir() {
                            hidden_dirs.push(path);
                        }
                        return None;
                    }

                    let display_name = if short_names {
                        path.file_name().map(|os| os.to_string_lossy().to_string())
                    } else if full_paths || absolute_path {
                        Some(path.to_string_lossy().to_string())
                    } else if let Some(prefix) = &prefix {
                        if let Ok(remainder) = path.strip_prefix(prefix) {
                            if directory {
                                // When the path is the same as the cwd, path_diff should be "."
                                let path_diff =
                                    if let Some(path_diff_not_dot) = diff_paths(&path, &cwd) {
                                        let path_diff_not_dot = path_diff_not_dot.to_string_lossy();
                                        if path_diff_not_dot.is_empty() {
                                            ".".to_string()
                                        } else {
                                            path_diff_not_dot.to_string()
                                        }
                                    } else {
                                        path.to_string_lossy().to_string()
                                    };

                                Some(path_diff)
                            } else {
                                let new_prefix = if let Some(pfx) = diff_paths(prefix, &cwd) {
                                    pfx
                                } else {
                                    prefix.to_path_buf()
                                };

                                Some(new_prefix.join(remainder).to_string_lossy().to_string())
                            }
                        } else {
                            Some(path.to_string_lossy().to_string())
                        }
                    } else {
                        Some(path.to_string_lossy().to_string())
                    }
                    .ok_or_else(|| {
                        ShellError::GenericError(
                            format!("Invalid file name: {:}", path.to_string_lossy()),
                            "invalid file name".into(),
                            Some(call_span),
                            None,
                            Vec::new(),
                        )
                    });

                    match display_name {
                        Ok(name) => {
                            let entry = dir_entry_dict(
                                &path,
                                &name,
                                metadata.as_ref(),
                                call_span,
                                long,
                                du,
                                ctrl_c.clone(),
                            );
                            match entry {
                                Ok(value) => Some(value),
                                Err(err) => Some(Value::Error { error: err }),
                            }
                        }
                        Err(err) => Some(Value::Error { error: err }),
                    }
                }
                _ => Some(Value::Nothing { span: call_span }),
            })
            .into_pipeline_data_with_metadata(
                PipelineMetadata {
                    data_source: DataSource::Ls,
                },
                engine_state.ctrlc.clone(),
            ))
    }
