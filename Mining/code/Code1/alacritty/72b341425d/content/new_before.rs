    pub fn new<T: EventListener>(
        term: &'a Term<T>,
        dfas: Option<&RegexSearch>,
        config: &'a Config<UiConfig>,
        colors: &'a List,
        show_cursor: bool,
    ) -> Self {
        let search = dfas.map(|dfas| RenderableSearch::new(&term, dfas)).unwrap_or_default();
        let terminal_content = term.renderable_content();

        // Copy the cursor and override its shape if necessary.
        let mut terminal_cursor = terminal_content.cursor;
        if !show_cursor {
            terminal_cursor.shape = CursorShape::Hidden;
        } else if !term.is_focused && config.cursor.unfocused_hollow {
            terminal_cursor.shape = CursorShape::HollowBlock;
        }

        Self { cursor: None, terminal_content, terminal_cursor, search, config, colors }
    }

    /// Viewport offset.
    pub fn display_offset(&self) -> usize {
        self.terminal_content.display_offset
    }

    /// Get the terminal cursor.
    pub fn cursor(mut self) -> Option<RenderableCursor> {
        // Drain the iterator to make sure the cursor is created.
        while self.next().is_some() && self.cursor.is_none() {}

        self.cursor
    }

    /// Get the RGB value for a color index.
    pub fn color(&self, color: usize) -> Rgb {
        self.terminal_content.colors[color].unwrap_or(self.colors[color])
    }

    /// Assemble the information required to render the terminal cursor.
    ///
    /// This will return `None` when there is no cursor visible.
    fn renderable_cursor(&mut self, cell: &RenderableCell) -> Option<RenderableCursor> {
        if self.terminal_cursor.shape == CursorShape::Hidden {
            return None;
        }

        // Expand across wide cell when inside wide char or spacer.
        let is_wide = if cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
            self.terminal_cursor.point.column -= 1;
            true
        } else {
            cell.flags.contains(Flags::WIDE_CHAR)
        };

        // Cursor colors.
        let color = if self.terminal_content.mode.contains(TermMode::VI) {
            self.config.ui_config.colors.vi_mode_cursor
        } else {
            self.config.ui_config.colors.cursor
        };
        let mut cursor_color =
            self.terminal_content.colors[NamedColor::Cursor].map_or(color.background, CellRgb::Rgb);
        let mut text_color = color.foreground;

        // Invert the cursor if it has a fixed background close to the cell's background.
        if matches!(
            cursor_color,
            CellRgb::Rgb(color) if color.contrast(cell.bg) < MIN_CURSOR_CONTRAST
        ) {
            cursor_color = CellRgb::CellForeground;
            text_color = CellRgb::CellBackground;
        }

        // Convert from cell colors to RGB.
        let text_color = text_color.color(cell.fg, cell.bg);
        let cursor_color = cursor_color.color(cell.fg, cell.bg);

        Some(RenderableCursor {
            point: self.terminal_cursor.point,
            shape: self.terminal_cursor.shape,
            cursor_color,
            text_color,
            is_wide,
        })
    }
}

impl<'a> Iterator for RenderableContent<'a> {
    type Item = RenderableCell;

    /// Gets the next renderable cell.
    ///
    /// Skips empty (background) cells and applies any flags to the cell state
    /// (eg. invert fg and bg colors).
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let cell = self.terminal_content.display_iter.next()?;
            let mut cell = RenderableCell::new(self, cell);

            if self.terminal_cursor.point == cell.point {
                // Store the cursor which should be rendered.
                self.cursor = self.renderable_cursor(&cell).map(|cursor| {
                    if cursor.shape == CursorShape::Block {
                        cell.fg = cursor.text_color;
                        cell.bg = cursor.cursor_color;

                        // Since we draw Block cursor by drawing cell below it with a proper color,
                        // we must adjust alpha to make it visible.
                        cell.bg_alpha = 1.;
                    }

                    cursor
                });

                return Some(cell);
            } else if !cell.is_empty() && !cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
                // Skip empty cells and wide char spacers.
                return Some(cell);
            }
        }
    }
}

/// Cell ready for rendering.
#[derive(Clone, Debug)]
pub struct RenderableCell {
    pub character: char,
    pub zerowidth: Option<Vec<char>>,
    pub point: Point,
    pub fg: Rgb,
    pub bg: Rgb,
    pub bg_alpha: f32,
    pub flags: Flags,
    pub is_match: bool,
}

impl RenderableCell {
    fn new<'a>(content: &mut RenderableContent<'a>, cell: Indexed<&Cell, Line>) -> Self {
        // Lookup RGB values.
        let mut fg_rgb = Self::compute_fg_rgb(content, cell.fg, cell.flags);
        let mut bg_rgb = Self::compute_bg_rgb(content, cell.bg);

        let mut bg_alpha = if cell.flags.contains(Flags::INVERSE) {
            mem::swap(&mut fg_rgb, &mut bg_rgb);
            1.0
        } else {
            Self::compute_bg_alpha(cell.bg)
        };

        let is_selected = content
            .terminal_content
            .selection
            .map_or(false, |selection| selection.contains_cell(&cell, content.terminal_cursor));
        let mut is_match = false;

        let colors = &content.config.ui_config.colors;
        if is_selected {
            let config_bg = colors.selection.background;
            let selected_fg = colors.selection.foreground.color(fg_rgb, bg_rgb);
            bg_rgb = config_bg.color(fg_rgb, bg_rgb);
            fg_rgb = selected_fg;

            if fg_rgb == bg_rgb && !cell.flags.contains(Flags::HIDDEN) {
                // Reveal inversed text when fg/bg is the same.
                fg_rgb = content.color(NamedColor::Background as usize);
                bg_rgb = content.color(NamedColor::Foreground as usize);
                bg_alpha = 1.0;
            } else if config_bg != CellRgb::CellBackground {
                bg_alpha = 1.0;
            }
        } else if content.search.advance(cell.point) {
            // Highlight the cell if it is part of a search match.
            let config_bg = colors.search.matches.background;
            let matched_fg = colors.search.matches.foreground.color(fg_rgb, bg_rgb);
            bg_rgb = config_bg.color(fg_rgb, bg_rgb);
            fg_rgb = matched_fg;

            if config_bg != CellRgb::CellBackground {
                bg_alpha = 1.0;
            }

            is_match = true;
        }

        RenderableCell {
            character: cell.c,
            zerowidth: cell.zerowidth().map(|zerowidth| zerowidth.to_vec()),
            point: cell.point,
            fg: fg_rgb,
            bg: bg_rgb,
            bg_alpha,
            flags: cell.flags,
            is_match,
        }
    }
