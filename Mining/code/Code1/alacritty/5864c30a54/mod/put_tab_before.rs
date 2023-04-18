    fn put_tab(&mut self, mut count: i64) {
        trace!("Putting tab: {}", count);

        while self.cursor.point.col < self.grid.num_cols() && count != 0 {
            count -= 1;

            let cell = &mut self.grid[&self.cursor.point];
            *cell = self.cursor.template;
            cell.c = self.cursor.charsets[self.active_charset].map('\t');

            loop {
                if (self.cursor.point.col + 1) == self.grid.num_cols() {
                    break;
                }

                self.cursor.point.col += 1;

                if self.tabs[self.cursor.point.col] {
                    break;
                }
            }
        }

        self.input_needs_wrap = false;
    }
