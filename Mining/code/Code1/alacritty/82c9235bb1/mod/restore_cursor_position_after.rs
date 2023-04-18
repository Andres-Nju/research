    fn restore_cursor_position(&mut self) {
        trace!("CursorRestore");
        let holder = if self.alt {
            &self.cursor_save_alt
        } else {
            &self.cursor_save
        };

        self.cursor = *holder;
        self.cursor.point.line = min(self.cursor.point.line, self.grid.num_lines() - 1);
        self.cursor.point.col = min(self.cursor.point.col, self.grid.num_cols() - 1);
    }
