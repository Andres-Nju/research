    fn restore_cursor_position(&mut self) {
        trace!("CursorRestore");
        let holder = if self.alt {
            &self.cursor_save_alt
        } else {
            &self.cursor_save
        };

        self.cursor = *holder;
    }
