    pub fn update_history(&mut self, history_size: usize, template: &T)
    {
        self.raw.update_history(history_size, Row::new(self.cols, &template));
        self.max_scroll_limit = history_size;
        self.scroll_limit = min(self.scroll_limit, history_size);
        self.display_offset = min(self.display_offset, self.scroll_limit);
    }
