    pub fn update_history(&mut self, history_size: usize, template: &T)
    {
        self.raw.update_history(history_size, Row::new(self.cols, &template));
        self.scroll_limit = min(self.scroll_limit, history_size);
    }
