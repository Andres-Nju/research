    fn event(
        &mut self,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut LapceTabData,
        env: &Env,
    ) {
        match event {
            Event::KeyDown(key_event) => {
                if ctx.is_focused() {
                    let mut keypress = data.keypress.clone();
                    let mut focus = LapceSettingsFocusData {
                        widget_id: self.widget_id,
                        editor_tab_id: self.editor_tab_id,
                        main_split: data.main_split.clone(),
                        config: data.config.clone(),
                    };
                    let mut_keypress = Arc::make_mut(&mut keypress);
                    let performed_action =
                        mut_keypress.key_down(ctx, key_event, &mut focus, env);
                    data.keypress = keypress;
                    data.main_split = focus.main_split;
                    if performed_action {
                        ctx.set_handled();
                    }
                }
            }
            Event::Command(cmd) if cmd.is(LAPCE_COMMAND) => {
                let cmd = cmd.get_unchecked(LAPCE_COMMAND);
                let mut focus = LapceSettingsFocusData {
                    widget_id: self.widget_id,
                    editor_tab_id: self.editor_tab_id,
                    main_split: data.main_split.clone(),
                    config: data.config.clone(),
                };
                if focus.run_command(ctx, cmd, None, Modifiers::empty(), env)
                    == CommandExecuted::Yes
                {
                    ctx.set_handled();
                }
                data.main_split = focus.main_split;
            }
            Event::Command(cmd) if cmd.is(LAPCE_UI_COMMAND) => {
                let command = cmd.get_unchecked(LAPCE_UI_COMMAND);
                match command {
                    LapceUICommand::Focus => {
                        ctx.set_handled();
                        self.request_focus(ctx, data);
                    }
                    LapceUICommand::ShowSettings => {
                        let kind = LapceSettingsKind::Core;
                        self.active = kind.clone();
                        self.switcher
                            .widget_mut()
                            .child_mut()
                            .set_active(kind, data);
                        ctx.request_focus();
                    }
                    LapceUICommand::ShowKeybindings => {
                        let kind = LapceSettingsKind::Keymap;
                        self.active = kind.clone();
                        self.switcher
                            .widget_mut()
                            .child_mut()
                            .set_active(kind, data);
                        ctx.request_focus();
                    }
                    LapceUICommand::ShowSettingsKind(kind) => {
                        self.active = kind.clone();
                        self.switcher
                            .widget_mut()
                            .child_mut()
                            .set_active(kind.clone(), data);
                        ctx.request_layout();
                    }
                    LapceUICommand::Hide => {
                        if let Some(active) = *data.main_split.active {
                            ctx.submit_command(Command::new(
                                LAPCE_UI_COMMAND,
                                LapceUICommand::Focus,
                                Target::Widget(active),
                            ));
                        }
                    }
                    LapceUICommand::VoltInstalled(_, _)
                    | LapceUICommand::VoltRemoved(_, _) => {
                        ctx.set_handled();
                        self.update_plugins(ctx, data);
                    }
                    _ => (),
                }
            }
            _ => {}
        }

        if ctx.is_handled() {
            return;
        }

        self.switcher.event(ctx, event, data, env);
        if event.should_propagate_to_hidden() {
            for child in self.children.values_mut() {
                child.event(ctx, event, data, env);
            }
        } else if let Some(child) = self.children.get_mut(&self.active) {
            child.event(ctx, event, data, env);
        }
    }
