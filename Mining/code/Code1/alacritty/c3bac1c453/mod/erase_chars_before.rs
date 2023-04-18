    fn erase_chars(&mut self, count: Column) {
        trace!("Erasing chars: count={}, col={}", count, self.cursor.point.col);
        let start = self.cursor.point.col;
        let end = min(start + count, self.grid.num_cols() - 1);

        let row = &mut self.grid[self.cursor.point.line];
        let template = self.cursor.template; // Cleared cells have current background color set
        for c in &mut row[start..end] {
            c.reset(&template);
        }
    }
