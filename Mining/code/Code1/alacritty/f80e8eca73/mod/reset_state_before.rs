    fn reset_state(&mut self) {
        if self.alt {
            self.swap_alt();
        }
        self.input_needs_wrap = false;
        self.cursor = Default::default();
        self.active_charset = Default::default();
        self.mode = Default::default();
        self.cursor_save = Default::default();
        self.cursor_save_alt = Default::default();
        self.colors = self.original_colors;
        self.color_modified = [false; color::COUNT];
        self.cursor_style = None;
        self.grid.reset(&Cell::default());
        self.alt_grid.reset(&Cell::default());
        self.scroll_region = Line(0)..self.grid.num_lines();
        self.title_stack = Vec::new();
        self.title = None;
    }
