    pub fn scroll<T: EventListener>(mut self, term: &Term<T>, lines: i32) -> Self {
        // Check number of lines the cursor needs to be moved.
        let overscroll = if lines > 0 {
            let max_scroll = term.history_size() - term.grid().display_offset();
            max(0, lines - max_scroll as i32)
        } else {
            let max_scroll = term.grid().display_offset();
            min(0, lines + max_scroll as i32)
        };

        // Clamp movement to within visible region.
        let line = (self.point.line - overscroll).grid_clamp(term, Boundary::Grid);

        // Find the first occupied cell after scrolling has been performed.
        let target_line = (self.point.line - lines).grid_clamp(term, Boundary::Grid);
        let column = first_occupied_in_line(term, target_line).unwrap_or_default().column;

        // Move cursor.
        self.point = Point::new(line, column);

        self
    }
