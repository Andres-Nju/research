    pub fn update_config<C>(&mut self, config: &Config<C>) {
        self.semantic_escape_chars = config.selection.semantic_escape_chars().to_owned();
        self.original_colors.fill_named(&config.colors);
        self.original_colors.fill_cube(&config.colors);
        self.original_colors.fill_gray_ramp(&config.colors);
        for i in 0..color::COUNT {
            if !self.color_modified[i] {
                self.colors[i] = self.original_colors[i];
            }
        }
        self.visual_bell.update_config(config);
        if let Some(0) = config.scrolling.faux_multiplier() {
            self.mode.remove(TermMode::ALTERNATE_SCROLL);
        }
        self.default_cursor_style = config.cursor.style;
        self.dynamic_title = config.dynamic_title();

        if self.alt {
            self.alt_grid.update_history(config.scrolling.history() as usize);
        } else {
            self.grid.update_history(config.scrolling.history() as usize);
        }
    }
