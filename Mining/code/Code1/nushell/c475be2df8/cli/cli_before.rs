pub async fn cli(
    mut syncer: EnvironmentSyncer,
    mut context: Context,
) -> Result<(), Box<dyn Error>> {
    #[cfg(windows)]
    const DEFAULT_COMPLETION_MODE: CompletionType = CompletionType::Circular;
    #[cfg(not(windows))]
    const DEFAULT_COMPLETION_MODE: CompletionType = CompletionType::List;

    let _ = load_plugins(&mut context);

    let config = Config::builder().color_mode(ColorMode::Forced).build();
    let mut rl: Editor<_> = Editor::with_config(config);

    // add key bindings to move over a whole word with Ctrl+ArrowLeft and Ctrl+ArrowRight
    rl.bind_sequence(
        KeyPress::ControlLeft,
        Cmd::Move(Movement::BackwardWord(1, Word::Vi)),
    );
    rl.bind_sequence(
        KeyPress::ControlRight,
        Cmd::Move(Movement::ForwardWord(1, At::AfterEnd, Word::Vi)),
    );

    #[cfg(windows)]
    {
        let _ = ansi_term::enable_ansi_support();
    }

    // we are ok if history does not exist
    let _ = rl.load_history(&History::path());

    let cc = context.ctrl_c.clone();
    ctrlc::set_handler(move || {
        cc.store(true, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");
    let mut ctrlcbreak = false;

    // before we start up, let's run our startup commands
    if let Ok(config) = crate::data::config::config(Tag::unknown()) {
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

        rl.set_helper(Some(crate::shell::Helper::new(context.clone())));

        let edit_mode = config::config(Tag::unknown())?
            .get("edit_mode")
            .map(|s| match s.value.expect_string() {
                "vi" => EditMode::Vi,
                "emacs" => EditMode::Emacs,
                _ => EditMode::Emacs,
            })
            .unwrap_or(EditMode::Emacs);

        rl.set_edit_mode(edit_mode);

        let key_timeout = config::config(Tag::unknown())?
            .get("key_timeout")
            .map(|s| s.value.expect_int())
            .unwrap_or(1);

        rl.set_keyseq_timeout(key_timeout as i32);

        let completion_mode = config::config(Tag::unknown())?
            .get("completion_mode")
            .map(|s| match s.value.expect_string() {
                "list" => CompletionType::List,
                "circular" => CompletionType::Circular,
                _ => DEFAULT_COMPLETION_MODE,
            })
            .unwrap_or(DEFAULT_COMPLETION_MODE);

        rl.set_completion_type(completion_mode);

        let colored_prompt = {
            #[cfg(feature = "starship-prompt")]
            {
                std::env::set_var("STARSHIP_SHELL", "");
                let mut starship_context =
                    starship::context::Context::new_with_dir(clap::ArgMatches::default(), cwd);

                match starship_context.config.config {
                    None => {
                        starship_context.config.config = create_default_starship_config();
                    }
                    Some(toml::Value::Table(t)) if t.is_empty() => {
                        starship_context.config.config = create_default_starship_config();
                    }
                    _ => {}
                };
                starship::print::get_prompt(starship_context)
            }
            #[cfg(not(feature = "starship-prompt"))]
            {
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
                rl.add_history_entry(line.clone());
                let _ = rl.save_history(&History::path());
                context.maybe_print_errors(Text::from(line));
            }

            LineResult::Error(line, err) => {
                rl.add_history_entry(line.clone());
                let _ = rl.save_history(&History::path());

                context.with_host(|host| {
                    print_err(err, host, &Text::from(line.clone()));
                });

                context.maybe_print_errors(Text::from(line.clone()));
            }

            LineResult::CtrlC => {
                let config_ctrlc_exit = config::config(Tag::unknown())?
                    .get("ctrlc_exit")
                    .map(|s| match s.value.expect_string() {
                        "true" => true,
                        _ => false,
                    })
                    .unwrap_or(false); // default behavior is to allow CTRL-C spamming similar to other shells

                if !config_ctrlc_exit {
                    continue;
                }

                if ctrlcbreak {
                    let _ = rl.save_history(&History::path());
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
    let _ = rl.save_history(&History::path());

    Ok(())
}
