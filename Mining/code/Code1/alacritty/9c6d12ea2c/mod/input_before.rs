    fn input(&mut self, c: char) {
        // If enabled, scroll to bottom when character is received
        if self.auto_scroll {
            self.scroll_display(Scroll::Bottom);
        }

        if self.input_needs_wrap {
            if !self.mode.contains(TermMode::LINE_WRAP) {
                return;
            }

            trace!("Wrapping input");

            {
                let location = Point { line: self.cursor.point.line, col: self.cursor.point.col };

                let cell = &mut self.grid[&location];
                cell.flags.insert(cell::Flags::WRAPLINE);
            }

            if (self.cursor.point.line + 1) >= self.scroll_region.end {
                self.linefeed();
            } else {
                self.cursor.point.line += 1;
            }

            self.cursor.point.col = Column(0);
            self.input_needs_wrap = false;
        }

        // Number of cells the char will occupy
        if let Some(width) = c.width() {
            let num_cols = self.grid.num_cols();

            // If in insert mode, first shift cells to the right.
            if self.mode.contains(TermMode::INSERT) && self.cursor.point.col + width < num_cols {
                let line = self.cursor.point.line;
                let col = self.cursor.point.col;
                let line = &mut self.grid[line];

                let src = line[col..].as_ptr();
                let dst = line[(col + width)..].as_mut_ptr();
                unsafe {
                    // memmove
                    ptr::copy(src, dst, (num_cols - col - width).0);
                }
            }

            // Handle zero-width characters
            if width == 0 {
                let col = self.cursor.point.col.0.saturating_sub(1);
                let line = self.cursor.point.line;
                if self.grid[line][Column(col)].flags.contains(cell::Flags::WIDE_CHAR_SPACER) {
                    col.saturating_sub(1);
                }
                self.grid[line][Column(col)].push_extra(c);
                return;
            }

            let cell = &mut self.grid[&self.cursor.point];
            *cell = self.cursor.template;
            cell.c = self.cursor.charsets[self.active_charset].map(c);

            // Handle wide chars
            if width == 2 {
                cell.flags.insert(cell::Flags::WIDE_CHAR);

                if self.cursor.point.col + 1 < num_cols {
                    self.cursor.point.col += 1;
                    let spacer = &mut self.grid[&self.cursor.point];
                    *spacer = self.cursor.template;
                    spacer.flags.insert(cell::Flags::WIDE_CHAR_SPACER);
                }
            }
        }

        if (self.cursor.point.col + 1) < self.grid.num_cols() {
            self.cursor.point.col += 1;
        } else {
            self.input_needs_wrap = true;
        }
    }
