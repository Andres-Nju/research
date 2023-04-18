    fn search_reset_state(&mut self) {
        // Unschedule pending timers.
        self.scheduler.unschedule(TimerId::DelayedSearch);

        // The viewport reset logic is only needed for vi mode, since without it our origin is
        // always at the current display offset instead of at the vi cursor position which we need
        // to recover to.
        if !self.terminal.mode().contains(TermMode::VI) {
            return;
        }

        // Reset display offset.
        self.terminal.scroll_display(Scroll::Delta(self.search_state.display_offset_delta));
        self.search_state.display_offset_delta = 0;

        // Clear focused match.
        self.search_state.focused_match = None;

        // Reset vi mode cursor.
        let mut origin = self.search_state.origin;
        origin.line = min(origin.line, self.terminal.screen_lines() - 1);
        origin.column = min(origin.column, self.terminal.cols() - 1);
        self.terminal.vi_mode_cursor.point = origin;

        *self.dirty = true;
    }
