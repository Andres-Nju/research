    fn event(
        &mut self,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut LapceTabData,
        env: &Env,
    ) {
        if let Some(find) = self.find.as_mut() {
            match event {
                Event::Command(cmd) if cmd.is(LAPCE_UI_COMMAND) => {}
                Event::Command(cmd) if cmd.is(LAPCE_COMMAND) => {}
                _ => {
                    if event.should_propagate_to_hidden() || data.find.visual {
                        find.event(ctx, event, data, env);
                    }
                }
            }
        }

        if ctx.is_handled() {
            return;
        }

        match event {
            Event::MouseDown(mouse_event) => match mouse_event.button {
                druid::MouseButton::Left => {
                    self.request_focus(ctx, data, true);
                }
                druid::MouseButton::Right => {
                    self.request_focus(ctx, data, false);
                }
                _ => (),
            },
            Event::Command(cmd) if cmd.is(LAPCE_UI_COMMAND) => {
                let command = cmd.get_unchecked(LAPCE_UI_COMMAND);
                if let LapceUICommand::Focus = command {
                    let editor_data = data.editor_view_content(self.view_id);
                    if data.config.editor.blink_interval > 0 {
                        self.cursor_blink_timer = ctx.request_timer(
                            Duration::from_millis(data.config.editor.blink_interval),
                            None,
                        );
                        *editor_data.editor.last_cursor_instant.borrow_mut() =
                            Instant::now();
                        ctx.request_paint();
                    }
                    self.request_focus(ctx, data, true);
                    self.ensure_cursor_visible(
                        ctx,
                        &editor_data,
                        &data.panel,
                        None,
                        env,
                    );
                }
            }
            Event::Timer(id) if self.cursor_blink_timer == *id => {
                ctx.set_handled();
                if data.config.editor.blink_interval > 0 {
                    if ctx.is_focused() {
                        ctx.request_paint();
                        self.cursor_blink_timer = ctx.request_timer(
                            Duration::from_millis(data.config.editor.blink_interval),
                            None,
                        );
                    } else {
                        self.cursor_blink_timer = TimerToken::INVALID;
                    }
                }
            }
            Event::Timer(id) if self.autosave_timer == *id => {
                ctx.set_handled();
                if let Some(editor) = data
                    .main_split
                    .active
                    .and_then(|active| data.main_split.editors.get(&active))
                    .cloned()
                {
                    if data.config.editor.autosave_interval > 0 {
                        if ctx.is_focused() {
                            let doc = data.main_split.editor_doc(self.view_id);
                            if !doc.buffer().is_pristine() {
                                ctx.submit_command(Command::new(
                                    LAPCE_COMMAND,
                                    LapceCommand {
                                        kind: CommandKind::Focus(FocusCommand::Save),
                                        data: None,
                                    },
                                    Target::Widget(editor.view_id),
                                ));
                            }
                            self.autosave_timer = ctx.request_timer(
                                Duration::from_millis(
                                    data.config.editor.autosave_interval,
                                ),
                                None,
                            );
                        } else {
                            self.cursor_blink_timer = TimerToken::INVALID;
                        }
                    }
                }
            }
            _ => {}
        }

        let editor = data.main_split.editors.get(&self.view_id).unwrap().clone();
        let mut editor_data = data.editor_view_content(self.view_id);
        let doc = editor_data.doc.clone();
        match event {
            Event::KeyDown(key_event) => {
                ctx.set_handled();
                if key_event.is_composing {
                    if data.config.editor.blink_interval > 0 {
                        self.cursor_blink_timer = ctx.request_timer(
                            Duration::from_millis(data.config.editor.blink_interval),
                            None,
                        );
                        *editor_data.editor.last_cursor_instant.borrow_mut() =
                            Instant::now();
                    }
                    if let Some(text) = self.ime.get_input_text() {
                        Arc::make_mut(&mut editor_data.doc).clear_ime_text();
                        editor_data.receive_char(ctx, &text);
                    } else if !self.ime.borrow().text().is_empty() {
                        let offset = editor_data.editor.cursor.offset();
                        let (line, col) =
                            editor_data.doc.buffer().offset_to_line_col(offset);
                        let doc = Arc::make_mut(&mut editor_data.doc);
                        doc.set_ime_pos(line, col, self.ime.get_shift());
                        doc.set_ime_text(self.ime.borrow().text().to_string());
                    } else {
                        Arc::make_mut(&mut editor_data.doc).clear_ime_text();
                    }
                } else {
                    Arc::make_mut(&mut editor_data.doc).clear_ime_text();
                    let mut keypress = data.keypress.clone();
                    if Arc::make_mut(&mut keypress).key_down(
                        ctx,
                        key_event,
                        &mut editor_data,
                        env,
                    ) {
                        self.ensure_cursor_visible(
                            ctx,
                            &editor_data,
                            &data.panel,
                            None,
                            env,
                        );
                    }
                    editor_data.sync_buffer_position(
                        self.editor.widget().editor.widget().inner().offset(),
                    );
                    editor_data.get_code_actions(ctx);

                    data.keypress = keypress.clone();
                }
            }
            Event::Command(cmd) if cmd.is(LAPCE_COMMAND) => {
                let command = cmd.get_unchecked(LAPCE_COMMAND);
                if editor_data.run_command(
                    ctx,
                    command,
                    None,
                    Modifiers::empty(),
                    env,
                ) == CommandExecuted::Yes
                {
                    ctx.set_handled();
                }

                // We don't want to send this on `FocusCommand::Save`, especially when autosave is enabled.
                if command.kind != CommandKind::Focus(FocusCommand::Save) {
                    self.ensure_cursor_visible(
                        ctx,
                        &editor_data,
                        &data.panel,
                        None,
                        env,
                    );
                }
            }
            Event::Command(cmd) if cmd.is(LAPCE_UI_COMMAND) => {
                let cmd = cmd.get_unchecked(LAPCE_UI_COMMAND);
                self.handle_lapce_ui_command(
                    ctx,
                    cmd,
                    &mut editor_data,
                    &data.panel,
                    env,
                );
            }
            _ => (),
        }
        data.update_from_editor_buffer_data(editor_data, &editor, &doc);

        self.header.event(ctx, event, data, env);
        self.editor.event(ctx, event, data, env);

        let offset = self.editor.widget().editor.widget().inner().offset();
        if editor.scroll_offset != offset {
            Arc::make_mut(data.main_split.editors.get_mut(&self.view_id).unwrap())
                .scroll_offset = offset;
        }
    }
