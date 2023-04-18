pub async fn cli(
    mut syncer: EnvironmentSyncer,
    mut context: Context,
) -> Result<(), Box<dyn Error>> {
    let configuration = nu_data::config::NuConfig::new();
    let history_path = crate::commands::history::history_path(&configuration);

    let (mut rl, config) = create_rustyline_configuration();

    // we are ok if history does not exist
    let _ = rl.load_history(&history_path);

    let skip_welcome_message = config
        .get("skip_welcome_message")
        .map(|x| x.is_true())
        .unwrap_or(false);
    if !skip_welcome_message {
        println!(
            "Welcome to Nushell {} (type 'help' for more info)",
            clap::crate_version!()
        );
    }

    #[cfg(windows)]
    {
        let _ = ansi_term::enable_ansi_support();
    }

    #[cfg(feature = "ctrlc")]
    {
        let cc = context.ctrl_c.clone();

        ctrlc::set_handler(move || {
            cc.store(true, Ordering::SeqCst);
        })
        .expect("Error setting Ctrl-C handler");
    }
    let mut ctrlcbreak = false;

    // before we start up, let's run our startup commands
    if let Ok(config) = nu_data::config::config(Tag::unknown()) {
        if let Some(commands) = config.get("startup") {
            match commands {
                Value {
                    value: UntaggedValue::Table(pipelines),
                    ..
                } => {
                    for pipeline in pipelines {
                        if let Ok(pipeline_string) = pipeline.as_string() {
                            let _ = run_pipeline_standalone(
                                pipeline_string,
                                false,
                                &mut context,
                                false,
                            )
                            .await;
                        }
                    }
                }
                _ => {
                    println!("warning: expected a table of pipeline strings as startup commands");
                }
            }
        }
    }

    loop {
        if context.ctrl_c.load(Ordering::SeqCst) {
            context.ctrl_c.store(false, Ordering::SeqCst);
            continue;
        }

        let cwd = context.shell_manager.path();

        let hinter = init_hinter(&config);

        rl.set_helper(Some(crate::shell::Helper::new(context.clone(), hinter)));

        let colored_prompt = {
            if let Some(prompt) = config.get("prompt") {
                let prompt_line = prompt.as_string()?;

                match nu_parser::lite_parse(&prompt_line, 0).map_err(ShellError::from) {
                    Ok(result) => {
                        let mut prompt_block =
                            nu_parser::classify_block(&result, context.registry());

                        let env = context.get_env();

                        prompt_block.block.expand_it_usage();

                        match run_block(
                            &prompt_block.block,
                            &mut context,
                            InputStream::empty(),
                            &Value::nothing(),
                            &IndexMap::new(),
                            &env,
                        )
                        .await
                        {
                            Ok(result) => match result.collect_string(Tag::unknown()).await {
                                Ok(string_result) => {
                                    let errors = context.get_errors();
                                    context.maybe_print_errors(Text::from(prompt_line));
                                    context.clear_errors();

                                    if !errors.is_empty() {
                                        "> ".to_string()
                                    } else {
                                        string_result.item
                                    }
                                }
                                Err(e) => {
                                    crate::cli::print_err(e, &Text::from(prompt_line));
                                    context.clear_errors();

                                    "> ".to_string()
                                }
                            },
                            Err(e) => {
                                crate::cli::print_err(e, &Text::from(prompt_line));
                                context.clear_errors();

                                "> ".to_string()
                            }
                        }
                    }
                    Err(e) => {
                        crate::cli::print_err(e, &Text::from(prompt_line));
                        context.clear_errors();

                        "> ".to_string()
                    }
                }
            } else {
                format!(
                    "\x1b[32m{}{}\x1b[m> ",
                    cwd,
                    match current_branch() {
                        Some(s) => format!("({})", s),
                        None => "".to_string(),
                    }
                )
            }
        };

        let prompt = {
            if let Ok(bytes) = strip_ansi_escapes::strip(&colored_prompt) {
                String::from_utf8_lossy(&bytes).to_string()
            } else {
                "> ".to_string()
            }
        };

        rl.helper_mut().expect("No helper").colored_prompt = colored_prompt;
        let mut initial_command = Some(String::new());
        let mut readline = Err(ReadlineError::Eof);
        while let Some(ref cmd) = initial_command {
            readline = rl.readline_with_initial(&prompt, (&cmd, ""));
            initial_command = None;
        }

        let line = process_line(readline, &mut context, false, true).await;

        // Check the config to see if we need to update the path
        // TODO: make sure config is cached so we don't path this load every call
        // FIXME: we probably want to be a bit more graceful if we can't set the environment
        syncer.reload();
        syncer.sync_env_vars(&mut context);
        syncer.sync_path_vars(&mut context);

        match line {
            LineResult::Success(line) => {
                rl.add_history_entry(&line);
                let _ = rl.save_history(&history_path);
                context.maybe_print_errors(Text::from(line));
            }

            LineResult::Error(line, err) => {
                rl.add_history_entry(&line);
                let _ = rl.save_history(&history_path);

                context.with_host(|_host| {
                    print_err(err, &Text::from(line.clone()));
                });

                context.maybe_print_errors(Text::from(line.clone()));
            }

            LineResult::CtrlC => {
                let config_ctrlc_exit = config::config(Tag::unknown())?
                    .get("ctrlc_exit")
                    .map(|s| s.value.is_true())
                    .unwrap_or(false); // default behavior is to allow CTRL-C spamming similar to other shells

                if !config_ctrlc_exit {
                    continue;
                }

                if ctrlcbreak {
                    let _ = rl.save_history(&history_path);
                    std::process::exit(0);
                } else {
                    context.with_host(|host| host.stdout("CTRL-C pressed (again to quit)"));
                    ctrlcbreak = true;
                    continue;
                }
            }

            LineResult::Break => {
                break;
            }
        }
        ctrlcbreak = false;
    }

    // we are ok if we can not save history
    let _ = rl.save_history(&history_path);

    Ok(())
}
