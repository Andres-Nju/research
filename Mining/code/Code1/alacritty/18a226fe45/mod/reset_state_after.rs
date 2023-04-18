    fn reset_state(&mut self) {
        if self.mode.contains(TermMode::ALT_SCREEN) {
            mem::swap(&mut self.grid, &mut self.inactive_grid);
        }
        self.active_charset = Default::default();
        self.colors = self.original_colors;
        self.color_modified = [false; color::COUNT];
        self.cursor_style = None;
        self.grid.reset();
        self.inactive_grid.reset();
        self.scroll_region = Line(0)..self.screen_lines();
        self.tabs = TabStops::new(self.cols());
        self.title_stack = Vec::new();
        self.title = None;
        self.selection = None;
        self.regex_search = None;

        // Preserve vi mode across resets.
        self.mode &= TermMode::VI;
        self.mode.insert(TermMode::default());
    }
