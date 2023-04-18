    pub fn retrieve_file<P: EditorPosition + Send + 'static>(
        &mut self,
        locations: Vec<(WidgetId, EditorLocation<P>)>,
        unsaved_buffer: Option<Rope>,
        cb: Option<InitBufferContentCb>,
    ) {
        if self.loaded || *self.load_started.borrow() {
            return;
        }

        *self.load_started.borrow_mut() = true;
        if let BufferContent::File(path) = &self.content {
            let id = self.id;
            let tab_id = self.tab_id;
            let path = path.clone();
            let event_sink = self.event_sink.clone();
            let proxy = self.proxy.clone();
            std::thread::spawn(move || {
                proxy.proxy_rpc.new_buffer(id, path.clone(), move |result| {
                    if let Ok(ProxyResponse::NewBufferResponse { content }) = result
                    {
                        let _ = event_sink.submit_command(
                            LAPCE_UI_COMMAND,
                            P::init_buffer_content_cmd(
                                path,
                                Rope::from(content),
                                locations,
                                unsaved_buffer,
                                cb,
                            ),
                            Target::Widget(tab_id),
                        );
                    };
                })
            });
        }

        self.retrieve_history("head");
    }

    pub fn retrieve_history(&mut self, version: &str) {
        if self.histories.contains_key(version) {
            return;
        }

        let history = DocumentHistory::new(version.to_string());
        history.retrieve(self);
        self.histories.insert(version.to_string(), history);
    }

    pub fn reload_history(&self, version: &str) {
        if let Some(history) = self.histories.get(version) {
            history.retrieve(self);
        }
    }

    pub fn load_history(&mut self, version: &str, content: Rope) {
        let mut history = DocumentHistory::new(version.to_string());
        history.load_content(content, self);
        self.histories.insert(version.to_string(), history);
    }

    pub fn get_history(&self, version: &str) -> Option<&DocumentHistory> {
        self.histories.get(version)
    }

    pub fn history_visual_line(&self, version: &str, line: usize) -> usize {
        let mut visual_line = 0;
        if let Some(history) = self.histories.get(version) {
            for (_i, change) in history.changes().iter().enumerate() {
                match change {
                    DiffLines::Left(range) => {
                        visual_line += range.len();
                    }
                    DiffLines::Both(_, r) | DiffLines::Right(r) => {
                        if r.contains(&line) {
                            visual_line += line - r.start;
                            break;
                        }
                        visual_line += r.len();
                    }
                    DiffLines::Skip(_, r) => {
                        if r.contains(&line) {
                            break;
                        }
                        visual_line += 1;
                    }
                }
            }
        }
        visual_line
    }

    pub fn history_actual_line_from_visual(
        &self,
        version: &str,
        visual_line: usize,
    ) -> usize {
        let mut current_visual_line = 0;
        let mut line = 0;
        if let Some(history) = self.histories.get(version) {
            for (i, change) in history.changes().iter().enumerate() {
                match change {
                    DiffLines::Left(range) => {
                        current_visual_line += range.len();
                        if current_visual_line > visual_line {
                            if let Some(change) = history.changes().get(i + 1) {
                                match change {
                                    DiffLines::Left(_) => {}
                                    DiffLines::Both(_, r)
                                    | DiffLines::Skip(_, r)
                                    | DiffLines::Right(r) => {
                                        line = r.start;
                                    }
                                }
                            } else if i > 0 {
                                if let Some(change) = history.changes().get(i - 1) {
                                    match change {
                                        DiffLines::Left(_) => {}
                                        DiffLines::Both(_, r)
                                        | DiffLines::Skip(_, r)
                                        | DiffLines::Right(r) => {
                                            line = r.end - 1;
                                        }
                                    }
                                }
                            }
                            break;
                        }
                    }
                    DiffLines::Skip(_, r) => {
                        current_visual_line += 1;
                        if current_visual_line > visual_line {
                            line = r.end;
                            break;
                        }
                    }
                    DiffLines::Both(_, r) | DiffLines::Right(r) => {
                        current_visual_line += r.len();
                        if current_visual_line > visual_line {
                            line = r.end - (current_visual_line - visual_line);
                            break;
                        }
                    }
                }
            }
        }
        if current_visual_line <= visual_line {
            self.buffer.last_line()
        } else {
            line
        }
    }

    fn trigger_head_change(&self) {
        if let Some(head) = self.histories.get("head") {
            head.trigger_update_change(self, history::DEFAULT_DIFF_EXTEND_LINES);
        }
    }

    pub fn trigger_history_change(&self, version: &str, extend_lines: usize) {
        if let Some(history) = self.histories.get(version) {
            history.trigger_update_change(self, extend_lines);
        }
    }

    pub fn update_history_changes(
        &mut self,
        rev: u64,
        version: &str,
        changes: Arc<Vec<DiffLines>>,
        diff_extend_lines: usize,
    ) {
        if rev != self.rev() {
            return;
        }
        if let Some(history) = self.histories.get_mut(version) {
            history.update_changes(changes, diff_extend_lines);
        }
    }

    pub fn update_history_styles(
        &mut self,
        version: &str,
        styles: Arc<Spans<Style>>,
    ) {
        if let Some(history) = self.histories.get_mut(version) {
            history.update_styles(styles);
        }
    }

    /// Request semantic styles for the buffer from the LSP through the proxy.
    fn get_semantic_styles(&self) {
        if !self.loaded() {
            return;
        }

        if !self.content().is_file() {
            return;
        }
        if let BufferContent::File(path) = self.content() {
            let tab_id = self.tab_id;
            let path = path.clone();
            let buffer_id = self.id();
            let buffer = self.buffer();
            let rev = buffer.rev();
            let len = buffer.len();
            let event_sink = self.event_sink.clone();
            let syntactic_styles =
                self.syntax().and_then(|s| s.styles.as_ref()).cloned();

            self.proxy
                .proxy_rpc
                .get_semantic_tokens(path.clone(), move |result| {
                    if let Ok(ProxyResponse::GetSemanticTokens { styles }) = result {
                        rayon::spawn(move || {
                            let mut styles_span = SpansBuilder::new(len);
                            for style in styles.styles {
                                styles_span.add_span(
                                    Interval::new(style.start, style.end),
                                    style.style,
                                );
                            }

                            let styles = styles_span.build();

                            let styles =
                                if let Some(syntactic_styles) = syntactic_styles {
                                    syntactic_styles.merge(&styles, |a, b| {
                                        if let Some(b) = b {
                                            return b.clone();
                                        }
                                        a.clone()
                                    })
                                } else {
                                    styles
                                };
                            let styles = Arc::new(styles);

                            let _ = event_sink.submit_command(
                                LAPCE_UI_COMMAND,
                                LapceUICommand::UpdateSemanticStyles {
                                    id: buffer_id,
                                    path,
                                    rev,
                                    styles,
                                },
                                Target::Widget(tab_id),
                            );
                        });
                    }
                });
        }
    }

    /// Request inlay hints for the buffer from the LSP through the proxy.
    pub fn get_inlay_hints(&self) {
        if !self.loaded() {
            return;
        }

        if !self.content().is_file() {
            return;
        }

        if let BufferContent::File(path) = self.content() {
            let tab_id = self.tab_id;
            let path = path.clone();
            let buffer = self.buffer().clone();
            let rev = buffer.rev();
            let len = buffer.len();
            let event_sink = self.event_sink.clone();
            self.proxy
                .proxy_rpc
                .get_inlay_hints(path.clone(), move |result| {
                    if let Ok(ProxyResponse::GetInlayHints { mut hints }) = result {
                        // Sort the inlay hints by their position, as the LSP does not guarantee that it will
                        // provide them in the order that they are in within the file
                        // as well, Spans does not iterate in the order that they appear
                        hints.sort_by(|left, right| {
                            left.position.cmp(&right.position)
                        });

                        let mut hints_span = SpansBuilder::new(len);
                        for hint in hints {
                            let offset =
                                buffer.offset_of_position(&hint.position).min(len);
                            hints_span.add_span(
                                Interval::new(offset, (offset + 1).min(len)),
                                hint,
                            );
                        }
                        let hints = hints_span.build();
                        let _ = event_sink.submit_command(
                            LAPCE_UI_COMMAND,
                            LapceUICommand::UpdateInlayHints { path, rev, hints },
                            Target::Widget(tab_id),
                        );
                    }
                });
        }
    }

    fn on_update(&mut self, edits: Option<SmallVec<[SyntaxEdit; 3]>>) {
        self.clear_code_actions();
        self.find.borrow_mut().unset();
        *self.find_progress.borrow_mut() = FindProgress::Started;
        self.get_inlay_hints();
        self.clear_style_cache();
        self.trigger_syntax_change(edits);
        self.get_semantic_styles();
        self.clear_sticky_headers_cache();
        self.trigger_head_change();
        self.notify_special();
    }

    /// Notify special buffer content's about their content potentially changing.
    fn notify_special(&self) {
        match &self.content {
            BufferContent::File(_) => {}
            BufferContent::Scratch(..) => {}
            BufferContent::Local(local) => {
                let s = self.buffer.to_string();
                match local {
                    LocalBufferKind::Search => {
                        let _ = self.event_sink.submit_command(
                            LAPCE_UI_COMMAND,
                            LapceUICommand::UpdateSearch(s, None),
                            Target::Widget(self.tab_id),
                        );
                    }
                    LocalBufferKind::PluginSearch => {}
                    LocalBufferKind::SourceControl => {}
                    LocalBufferKind::BranchesFilter => {}
                    LocalBufferKind::Empty => {}
                    LocalBufferKind::Rename => {}
                    LocalBufferKind::Palette => {
                        let _ = self.event_sink.submit_command(
                            LAPCE_UI_COMMAND,
                            LapceUICommand::UpdatePaletteInput(s),
                            Target::Widget(self.tab_id),
                        );
                    }
                    LocalBufferKind::FilePicker => {
                        let pwd = PathBuf::from(s);
                        let _ = self.event_sink.submit_command(
                            LAPCE_UI_COMMAND,
                            LapceUICommand::UpdatePickerPwd(pwd),
                            Target::Widget(self.tab_id),
                        );
                    }
                    LocalBufferKind::Keymap => {
                        let _ = self.event_sink.submit_command(
                            LAPCE_UI_COMMAND,
                            LapceUICommand::UpdateKeymapsFilter(s),
                            Target::Widget(self.tab_id),
                        );
                    }
                    LocalBufferKind::Settings => {
                        let _ = self.event_sink.submit_command(
                            LAPCE_UI_COMMAND,
                            LapceUICommand::UpdateSettingsFilter(s),
                            Target::Widget(self.tab_id),
                        );
                    }
                    LocalBufferKind::PathName => {
                        // TODO: anything to update with this?
                    }
                }
            }
            BufferContent::SettingsValue(..) => {}
        }
    }

    pub fn set_syntax(&mut self, syntax: Option<Syntax>) {
        self.syntax = syntax;
        if self.semantic_styles.is_none() {
            self.clear_style_cache();
        }
        self.clear_sticky_headers_cache();
    }

    fn clear_sticky_headers_cache(&self) {
        self.sticky_headers.borrow_mut().clear();
    }

    pub fn set_semantic_styles(&mut self, styles: Option<Arc<Spans<Style>>>) {
        self.semantic_styles = styles;
        self.clear_style_cache();
    }

    fn clear_style_cache(&self) {
        self.line_styles.borrow_mut().clear();
        self.clear_text_layout_cache();
    }

    fn clear_text_layout_cache(&self) {
        self.text_layouts.borrow_mut().clear();
    }

    fn clear_code_actions(&mut self) {
        self.code_actions.clear();
    }

    pub fn trigger_syntax_change(
        &mut self,
        edits: Option<SmallVec<[SyntaxEdit; 3]>>,
    ) {
        let Some(syntax)  = self.syntax.as_mut() else { return };

        let rev = self.buffer.rev();
        let text = self.buffer.text().clone();

        syntax.parse(rev, text, edits.as_deref());
    }

    /// Update the inlay hints with new ones
    /// Clears any caches that need to be updated after change
    pub fn set_inlay_hints(&mut self, hints: Spans<InlayHint>) {
        self.inlay_hints = Some(hints);
        self.clear_text_layout_cache();
    }

    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    pub fn buffer_mut(&mut self) -> &mut Buffer {
        &mut self.buffer
    }

    pub fn syntax(&self) -> Option<&Syntax> {
        self.syntax.as_ref()
    }

    /// Update the styles after an edit, so the highlights are at the correct positions.  
    /// This does not do a reparse of the document itself.
    fn update_styles(&mut self, delta: &RopeDelta) {
        if let Some(styles) = self.semantic_styles.as_mut() {
            Arc::make_mut(styles).apply_shape(delta);
        }
        if let Some(syntax) = self.syntax.as_mut() {
            if let Some(styles) = syntax.styles.as_mut() {
                Arc::make_mut(styles).apply_shape(delta);
            }
        }

        if let Some(syntax) = self.syntax.as_mut() {
            syntax.lens.apply_delta(delta);
        }
    }

    /// Update the inlay hints so their positions are correct after an edit.
    fn update_inlay_hints(&mut self, delta: &RopeDelta) {
        if let Some(hints) = self.inlay_hints.as_mut() {
            hints.apply_shape(delta);
        }
    }

    pub fn set_ime_pos(&mut self, line: usize, col: usize, shift: usize) {
        self.ime_pos = (line, col, shift);
    }

    pub fn ime_text(&self) -> Option<&Arc<str>> {
        self.ime_text.as_ref()
    }

    pub fn ime_pos(&self) -> (usize, usize, usize) {
        self.ime_pos
    }

    pub fn set_ime_text(&mut self, text: &str) {
        self.ime_text = Some(Arc::from(text));
        self.clear_text_layout_cache();
    }

    pub fn clear_ime_text(&mut self) {
        if self.ime_text.is_some() {
            self.ime_text = None;
            self.clear_text_layout_cache();
        }
    }

    /// Get the phantom text for a given line
    pub fn line_phantom_text(
        &self,
        config: &LapceConfig,
        line: usize,
    ) -> PhantomTextLine {
        let start_offset = self.buffer.offset_of_line(line);
        let end_offset = self.buffer.offset_of_line(line + 1);

        // If hints are enabled, and the hints field is filled, then get the hints for this line
        // and convert them into PhantomText instances
        let hints = config
            .editor
            .enable_inlay_hints
            .then_some(())
            .and(self.inlay_hints.as_ref())
            .map(|hints| hints.iter_chunks(start_offset..end_offset))
            .into_iter()
            .flatten()
            .filter(|(interval, _)| {
                interval.start >= start_offset && interval.start < end_offset
            })
            .map(|(interval, inlay_hint)| {
                let (_, col) = self.buffer.offset_to_line_col(interval.start);
                let text = match &inlay_hint.label {
                    InlayHintLabel::String(label) => label.to_string(),
                    InlayHintLabel::LabelParts(parts) => {
                        parts.iter().map(|p| &p.value).join("")
                    }
                };
                PhantomText {
                    kind: PhantomTextKind::InlayHint,
                    col,
                    text,
                    fg: Some(
                        config
                            .get_color_unchecked(LapceTheme::INLAY_HINT_FOREGROUND)
                            .clone(),
                    ),
                    font_family: Some(config.editor.inlay_hint_font_family()),
                    font_size: Some(config.editor.inlay_hint_font_size()),
                    bg: Some(
                        config
                            .get_color_unchecked(LapceTheme::INLAY_HINT_BACKGROUND)
                            .clone(),
                    ),
                    under_line: None,
                }
            });
        // You're quite unlikely to have more than six hints on a single line
        // this later has the diagnostics added onto it, but that's still likely to be below six
        // overall.
        let mut text: SmallVec<[PhantomText; 6]> = hints.collect();

        // The max severity is used to determine the color given to the background of the line
        let mut max_severity = None;
        // If error lens is enabled, and the diagnostics field is filled, then get the diagnostics
        // that end on this line which have a severity worse than HINT and convert them into
        // PhantomText instances
        let diag_text = config
            .editor
            .enable_error_lens
            .then_some(())
            .and(self.diagnostics.as_ref())
            .map(|x| x.iter())
            .into_iter()
            .flatten()
            .filter(|diag| {
                diag.diagnostic.range.end.line as usize == line
                    && diag.diagnostic.severity < Some(DiagnosticSeverity::HINT)
            })
            .map(|diag| {
                match (diag.diagnostic.severity, max_severity) {
                    (Some(severity), Some(max)) => {
                        if severity < max {
                            max_severity = Some(severity);
                        }
                    }
                    (Some(severity), None) => {
                        max_severity = Some(severity);
                    }
                    _ => {}
                }

                let rope_text = self.buffer.rope_text();
                let col = rope_text.offset_of_line(line + 1)
                    - rope_text.offset_of_line(line);
                let fg = {
                    let severity = diag
                        .diagnostic
                        .severity
                        .unwrap_or(DiagnosticSeverity::WARNING);
                    let theme_prop = if severity == DiagnosticSeverity::ERROR {
                        LapceTheme::ERROR_LENS_ERROR_FOREGROUND
                    } else if severity == DiagnosticSeverity::WARNING {
                        LapceTheme::ERROR_LENS_WARNING_FOREGROUND
                    } else {
                        // information + hint (if we keep that) + things without a severity
                        LapceTheme::ERROR_LENS_OTHER_FOREGROUND
                    };

                    config.get_color_unchecked(theme_prop).clone()
                };
                let text =
                    format!("    {}", diag.diagnostic.message.lines().join(" "));
                PhantomText {
                    kind: PhantomTextKind::Diagnostic,
                    col,
                    text,
                    fg: Some(fg),
                    font_size: Some(config.editor.error_lens_font_size()),
                    font_family: Some(config.editor.error_lens_font_family()),
                    bg: None,
                    under_line: None,
                }
            });
        let mut diag_text: SmallVec<[PhantomText; 6]> = diag_text.collect();

        text.append(&mut diag_text);

        let (completion_line, completion_col) = self.completion_pos;
        let completion_text = config
            .editor
            .enable_completion_lens
            .then_some(())
            .and(self.completion.as_ref())
            // TODO: We're probably missing on various useful completion things to include here!
            .filter(|_| line == completion_line)
            .map(|completion| PhantomText {
                kind: PhantomTextKind::Completion,
                col: completion_col,
                text: completion.to_string(),
                fg: Some(
                    config
                        .get_color_unchecked(LapceTheme::COMPLETION_LENS_FOREGROUND)
                        .clone(),
                ),
                font_size: Some(config.editor.completion_lens_font_size()),
                font_family: Some(config.editor.completion_lens_font_family()),
                bg: None,
                under_line: None,
                // TODO: italics?
            });
        if let Some(completion_text) = completion_text {
            text.push(completion_text);
        }

        if let Some(ime_text) = self.ime_text.as_ref() {
            let (ime_line, col, _) = self.ime_pos;
            if line == ime_line {
                text.push(PhantomText {
                    kind: PhantomTextKind::Ime,
                    text: ime_text.to_string(),
                    col,
                    font_size: None,
                    font_family: None,
                    fg: None,
                    bg: None,
                    under_line: Some(
                        config
                            .get_color_unchecked(LapceTheme::EDITOR_FOREGROUND)
                            .clone(),
                    ),
                });
            }
        }

        text.sort_by(|a, b| {
            if a.col == b.col {
                a.kind.cmp(&b.kind)
            } else {
                a.col.cmp(&b.col)
            }
        });

        PhantomTextLine { text, max_severity }
    }

    fn apply_deltas(&mut self, deltas: &[(RopeDelta, InvalLines, SyntaxEdit)]) {
        let rev = self.rev() - deltas.len() as u64;
        for (i, (delta, _, _)) in deltas.iter().enumerate() {
            self.update_styles(delta);
            self.update_inlay_hints(delta);
            self.update_diagnostics(delta);
            self.update_completion(delta);
            if let BufferContent::File(path) = &self.content {
                self.proxy.proxy_rpc.update(
                    path.clone(),
                    delta.clone(),
                    rev + i as u64 + 1,
                );
            }
        }

        // TODO(minor): We could avoid this potential allocation since most apply_delta callers are actually using a Vec
        // which we could reuse.
        // We use a smallvec because there is unlikely to be more than a couple of deltas
        let edits = deltas.iter().map(|(_, _, edits)| edits.clone()).collect();
        self.on_update(Some(edits));
    }

    pub fn do_insert(
        &mut self,
        cursor: &mut Cursor,
        s: &str,
        config: &LapceConfig,
    ) -> Vec<(RopeDelta, InvalLines, SyntaxEdit)> {
        let old_cursor = cursor.mode.clone();
        let deltas = Editor::insert(
            cursor,
            &mut self.buffer,
            s,
            self.syntax.as_ref(),
            config.editor.auto_closing_matching_pairs,
        );
        // Keep track of the change in the cursor mode for undo/redo
        self.buffer_mut().set_cursor_before(old_cursor);
        self.buffer_mut().set_cursor_after(cursor.mode.clone());
        self.apply_deltas(&deltas);
        deltas
    }

    pub fn do_raw_edit(
        &mut self,
        edits: &[(impl AsRef<Selection>, &str)],
        edit_type: EditType,
    ) -> (RopeDelta, InvalLines, SyntaxEdit) {
        let (delta, inval_lines, edits) = self.buffer.edit(edits, edit_type);
        self.apply_deltas(&[(delta.clone(), inval_lines.clone(), edits.clone())]);
        (delta, inval_lines, edits)
    }

    pub fn do_edit(
        &mut self,
        cursor: &mut Cursor,
        cmd: &EditCommand,
        modal: bool,
        register: &mut Register,
    ) -> Vec<(RopeDelta, InvalLines, SyntaxEdit)> {
        let mut clipboard = SystemClipboard {};
        let old_cursor = cursor.mode.clone();
        let deltas = Editor::do_edit(
            cursor,
            &mut self.buffer,
            cmd,
            self.syntax.as_ref(),
            &mut clipboard,
            modal,
            register,
        );

        if !deltas.is_empty() {
            self.buffer_mut().set_cursor_before(old_cursor);
            self.buffer_mut().set_cursor_after(cursor.mode.clone());
        }

        self.apply_deltas(&deltas);
        deltas
    }

    pub fn do_multi_selection(
        &self,
        text: &mut PietText,
        cursor: &mut Cursor,
        cmd: &MultiSelectionCommand,
        view: &EditorView,
        config: &LapceConfig,
    ) {
        use MultiSelectionCommand::*;
        match cmd {
            SelectUndo => {
                if let CursorMode::Insert(_) = cursor.mode.clone() {
                    if let Some(selection) =
                        cursor.history_selections.last().cloned()
                    {
                        cursor.mode = CursorMode::Insert(selection);
                    }
                    cursor.history_selections.pop();
                }
            }
            InsertCursorAbove => {
                if let CursorMode::Insert(mut selection) = cursor.mode.clone() {
                    let offset = selection.first().map(|s| s.end).unwrap_or(0);
                    let (new_offset, _) = self.move_offset(
                        text,
                        offset,
                        cursor.horiz.as_ref(),
                        1,
                        &Movement::Up,
                        Mode::Insert,
                        view,
                        config,
                    );
                    if new_offset != offset {
                        selection.add_region(SelRegion::new(
                            new_offset, new_offset, None,
                        ));
                    }
                    cursor.set_insert(selection);
                }
            }
            InsertCursorBelow => {
                if let CursorMode::Insert(mut selection) = cursor.mode.clone() {
                    let offset = selection.last().map(|s| s.end).unwrap_or(0);
                    let (new_offset, _) = self.move_offset(
                        text,
                        offset,
                        cursor.horiz.as_ref(),
                        1,
                        &Movement::Down,
                        Mode::Insert,
                        view,
                        config,
                    );
                    if new_offset != offset {
                        selection.add_region(SelRegion::new(
                            new_offset, new_offset, None,
                        ));
                    }
                    cursor.set_insert(selection);
                }
            }
            InsertCursorEndOfLine => {
                if let CursorMode::Insert(selection) = cursor.mode.clone() {
                    let mut new_selection = Selection::new();
                    for region in selection.regions() {
                        let (start_line, _) =
                            self.buffer.offset_to_line_col(region.min());
                        let (end_line, end_col) =
                            self.buffer.offset_to_line_col(region.max());
                        for line in start_line..end_line + 1 {
                            let offset = if line == end_line {
                                self.buffer.offset_of_line_col(line, end_col)
                            } else {
                                self.buffer.line_end_offset(line, true)
                            };
                            new_selection
                                .add_region(SelRegion::new(offset, offset, None));
                        }
                    }
                    cursor.set_insert(new_selection);
                }
            }
            SelectCurrentLine => {
                if let CursorMode::Insert(selection) = cursor.mode.clone() {
                    let mut new_selection = Selection::new();
                    for region in selection.regions() {
                        let start_line = self.buffer.line_of_offset(region.min());
                        let start = self.buffer.offset_of_line(start_line);
                        let end_line = self.buffer.line_of_offset(region.max());
                        let end = self.buffer.offset_of_line(end_line + 1);
                        new_selection.add_region(SelRegion::new(start, end, None));
                    }
                    cursor.set_insert(selection);
                }
            }
            SelectAllCurrent => {
                if let CursorMode::Insert(mut selection) = cursor.mode.clone() {
                    if !selection.is_empty() {
                        let first = selection.first().unwrap();
                        let (start, end) = if first.is_caret() {
                            self.buffer.select_word(first.start)
                        } else {
                            (first.min(), first.max())
                        };
                        let search_str = self.buffer.slice_to_cow(start..end);
                        let case_sensitive = self.find.borrow().case_sensitive();
                        let multicursor_case_sensitive =
                            config.editor.multicursor_case_sensitive;
                        let case_sensitive =
                            multicursor_case_sensitive || case_sensitive;
                        let search_whole_word =
                            config.editor.multicursor_whole_words;
                        let mut find = Find::new(0);
                        find.set_case_sensitive(case_sensitive);
                        find.set_find(&search_str, false, search_whole_word);
                        let mut offset = 0;
                        while let Some((start, end)) =
                            find.next(self.buffer.text(), offset, false, false)
                        {
                            offset = end;
                            selection.add_region(SelRegion::new(start, end, None));
                        }
                    }
                    cursor.set_insert(selection);
                }
            }
            SelectNextCurrent => {
                if let CursorMode::Insert(mut selection) = cursor.mode.clone() {
                    if !selection.is_empty() {
                        let mut had_caret = false;
                        for region in selection.regions_mut() {
                            if region.is_caret() {
                                had_caret = true;
                                let (start, end) =
                                    self.buffer.select_word(region.start);
                                region.start = start;
                                region.end = end;
                            }
                        }
                        if !had_caret {
                            let r = selection.last_inserted().unwrap();
                            let search_str =
                                self.buffer.slice_to_cow(r.min()..r.max());
                            let case_sensitive = self.find.borrow().case_sensitive();
                            let case_sensitive =
                                config.editor.multicursor_case_sensitive
                                    || case_sensitive;
                            let search_whole_word =
                                config.editor.multicursor_whole_words;
                            let mut find = Find::new(0);
                            find.set_case_sensitive(case_sensitive);
                            find.set_find(&search_str, false, search_whole_word);
                            let mut offset = r.max();
                            let mut seen = HashSet::new();
                            while let Some((start, end)) =
                                find.next(self.buffer.text(), offset, false, true)
                            {
                                if !selection
                                    .regions()
                                    .iter()
                                    .any(|r| r.min() == start && r.max() == end)
                                {
                                    selection.add_region(SelRegion::new(
                                        start, end, None,
                                    ));
                                    break;
                                }
                                if seen.contains(&end) {
                                    break;
                                }
                                offset = end;
                                seen.insert(offset);
                            }
                        }
                    }
                    cursor.set_insert(selection);
                }
            }
            SelectSkipCurrent => {
                if let CursorMode::Insert(mut selection) = cursor.mode.clone() {
                    if !selection.is_empty() {
                        let r = selection.last_inserted().unwrap();
                        if r.is_caret() {
                            let (start, end) = self.buffer.select_word(r.start);
                            selection.replace_last_inserted_region(SelRegion::new(
                                start, end, None,
                            ));
                        } else {
                            let search_str =
                                self.buffer.slice_to_cow(r.min()..r.max());
                            let case_sensitive = self.find.borrow().case_sensitive();
                            let mut find = Find::new(0);
                            find.set_case_sensitive(case_sensitive);
                            find.set_find(&search_str, false, false);
                            let mut offset = r.max();
                            let mut seen = HashSet::new();
                            while let Some((start, end)) =
                                find.next(self.buffer.text(), offset, false, true)
                            {
                                if !selection
                                    .regions()
                                    .iter()
                                    .any(|r| r.min() == start && r.max() == end)
                                {
                                    selection.replace_last_inserted_region(
                                        SelRegion::new(start, end, None),
                                    );
                                    break;
                                }
                                if seen.contains(&end) {
                                    break;
                                }
                                offset = end;
                                seen.insert(offset);
                            }
                        }
                    }
                    cursor.set_insert(selection);
                }
            }
            SelectAll => {
                let new_selection = Selection::region(0, self.buffer.len());
                cursor.set_insert(new_selection);
            }
        }
    }

    pub fn do_motion_mode(
        &mut self,
        cursor: &mut Cursor,
        motion_mode: MotionMode,
        register: &mut Register,
    ) {
        if let Some(m) = &cursor.motion_mode {
            if m == &motion_mode {
                let offset = cursor.offset();
                let deltas = Editor::execute_motion_mode(
                    cursor,
                    &mut self.buffer,
                    motion_mode,
                    offset,
                    offset,
                    true,
                    register,
                );
                self.apply_deltas(&deltas);
            }
            cursor.motion_mode = None;
        } else {
            cursor.motion_mode = Some(motion_mode);
        }
    }

    pub fn do_paste(&mut self, cursor: &mut Cursor, data: &RegisterData) {
        let deltas = Editor::do_paste(cursor, &mut self.buffer, data);
        self.apply_deltas(&deltas)
    }

    /// Get the active style information, either the semantic styles or the
    /// tree-sitter syntax styles.
    pub fn styles(&self) -> Option<&Arc<Spans<Style>>> {
        if let Some(semantic_styles) = self.semantic_styles.as_ref() {
            Some(semantic_styles)
        } else {
            self.syntax().and_then(|s| s.styles.as_ref())
        }
    }

    /// Get the style information for the particular line from semantic/syntax highlighting.  
    /// This caches the result if possible.
    fn line_style(&self, line: usize) -> Arc<Vec<LineStyle>> {
        if self.line_styles.borrow().get(&line).is_none() {
            let styles = self.styles();

            let line_styles = styles
                .map(|styles| line_styles(self.buffer.text(), line, styles))
                .unwrap_or_default();
            self.line_styles
                .borrow_mut()
                .insert(line, Arc::new(line_styles));
        }
        self.line_styles.borrow().get(&line).cloned().unwrap()
    }

    /// Get the (line, col) of a particular point within the editor.
    /// The boolean indicates whether the point is within the text bounds.  
    /// Points outside of vertical bounds will return the last line.
    /// Points outside of horizontal bounds will return the last column on the line.
    pub fn line_col_of_point(
        &self,
        text: &mut PietText,
        mode: Mode,
        point: Point,
        view: &EditorView,
        config: &LapceConfig,
    ) -> ((usize, usize), bool) {
        let (line, font_size) = match view {
            EditorView::Diff(version) => {
                let changes = self
                    .get_history(version)
                    .map(|h| h.changes())
                    .unwrap_or_default();
                let line_height = config.editor.line_height();
                // Tracks the actual line in the file.
                let mut line = 0;
                // Tracks the lines that are displayed in the editor.
                let mut lines = 0;
                for change in changes {
                    match change {
                        DiffLines::Left(l) => {
                            lines += l.len();
                            if (lines * line_height) as f64 > point.y {
                                break;
                            }
                        }
                        DiffLines::Skip(_l, r) => {
                            // Skip only has one line rendered, so we only update this by 1
                            lines += 1;
                            if (lines * line_height) as f64 > point.y {
                                break;
                            }
                            // However, skip moves forward multiple lines in the underlying
                            // file so we need to update this.
                            line += r.len();
                        }
                        DiffLines::Both(_, r) | DiffLines::Right(r) => {
                            lines += r.len();
                            if (lines * line_height) as f64 > point.y {
                                line += ((point.y
                                    - ((lines - r.len()) * line_height) as f64)
                                    / line_height as f64)
                                    .floor()
                                    as usize;
                                break;
                            }
                            line += r.len();
                        }
                    }
                }
                (line, config.editor.font_size)
            }
            EditorView::Lens => {
                if let Some(syntax) = self.syntax() {
                    // If we have a syntax, then we need to do logic which handles that some text
                    // will be the smaller lens font size, and some text will be larger (like
                    // function names). We can just use the utility functions on the lens for this.
                    let lens = &syntax.lens;
                    let line = lens.line_of_height(point.y.round() as usize);
                    let line_height =
                        lens.height_of_line(line + 1) - lens.height_of_line(line);
                    let font_size = if line_height < config.editor.line_height() {
                        config.editor.code_lens_font_size
                    } else {
                        config.editor.font_size
                    };
                    (line, font_size)
                } else {
                    // The entire file is small, so we can just do a division.
                    (
                        (point.y / config.editor.line_height() as f64).floor()
                            as usize,
                        config.editor.font_size,
                    )
                }
            }
            EditorView::Normal => (
                (point.y / config.editor.line_height() as f64).floor() as usize,
                config.editor.font_size,
            ),
        };

        let line = line.min(self.buffer.last_line());

        let mut x_shift = 0.0;
        if font_size < config.editor.font_size {
            let line_content = self.buffer.line_content(line);
            let mut col = 0usize;
            for ch in line_content.chars() {
                if ch == ' ' || ch == '\t' {
                    col += 1;
                } else {
                    break;
                }
            }

            // If there's indentation, then we look at the difference between the normal text
            // and the shrunk text to shift the point over.
            if col > 0 {
                let normal_text_layout = self.get_text_layout(
                    text,
                    line,
                    config.editor.font_size,
                    config,
                );
                let small_text_layout =
                    self.get_text_layout(text, line, font_size, config);
                x_shift =
                    normal_text_layout.text.hit_test_text_position(col).point.x
                        - small_text_layout.text.hit_test_text_position(col).point.x;
            }
        }

        // Since we have the line, we can do a hit test after shifting the point to be within the
        // line itself
        let text_layout = self.get_text_layout(text, line, font_size, config);
        let hit_point = text_layout
            .text
            .hit_test_point(Point::new(point.x - x_shift, 0.0));
        // We have to unapply the phantom text shifting in order to get back to the column in
        // the actual buffer
        let phantom_text = self.line_phantom_text(config, line);
        let col = phantom_text.before_col(hit_point.idx);
        // Ensure that the column doesn't end up out of bounds, so things like clicking on the far
        // right end will just go to the end of the line.
        let max_col = self.buffer.line_end_col(line, mode != Mode::Normal);
        let mut col = col.min(max_col);

        if config.editor.atomic_soft_tabs && config.editor.tab_width > 1 {
            col = snap_to_soft_tab_line_col(
                &self.buffer,
                line,
                col,
                SnapDirection::Nearest,
                config.editor.tab_width,
            );
        }

        ((line, col), hit_point.is_inside)
    }

    /// Get the offset of a particular point within the editor.  
    /// The boolean indicates whether the point is inside the text or not
    /// Points outside of vertical bounds will return the last line.
    /// Points outside of horizontal bounds will return the last column on the line.
    pub fn offset_of_point(
        &self,
        text: &mut PietText,
        mode: Mode,
        point: Point,
        view: &EditorView,
        config: &LapceConfig,
    ) -> (usize, bool) {
        let ((line, col), is_inside) =
            self.line_col_of_point(text, mode, point, view, config);
        (self.buffer.offset_of_line_col(line, col), is_inside)
    }

    /// Get the (point above, point below) of a particular offset within the editor.
    pub fn points_of_offset(
        &self,
        text: &mut PietText,
        offset: usize,
        view: &EditorView,
        config: &LapceConfig,
    ) -> (Point, Point) {
        let (line, col) = self.buffer.offset_to_line_col(offset);
        self.points_of_line_col(text, line, col, view, config)
    }

    /// Get the (point above, point below) of a particular (line, col) within the editor.
    pub fn points_of_line_col(
        &self,
        text: &mut PietText,
        line: usize,
        col: usize,
        view: &EditorView,
        config: &LapceConfig,
    ) -> (Point, Point) {
        let (y, line_height, font_size) = match view {
            EditorView::Diff(version) => {
                let changes = self
                    .get_history(version)
                    .map(|h| h.changes())
                    .unwrap_or_default();
                let line_height = config.editor.line_height();
                let mut current_line = 0;
                let mut y = 0;
                for change in changes {
                    match change {
                        DiffLines::Left(l) => {
                            y += l.len() * line_height;
                        }
                        DiffLines::Skip(_l, r) => {
                            if current_line + r.len() > line {
                                break;
                            }
                            y += line_height;
                            current_line += r.len();
                        }
