    fn shrink_cols(&mut self, reflow: bool, cols: Column) {
        self.cols = cols;

        // Remove the linewrap special case, by moving the cursor outside of the grid.
        if self.cursor.input_needs_wrap && reflow {
            self.cursor.input_needs_wrap = false;
            self.cursor.point.col += 1;
        }

        let mut new_raw = Vec::with_capacity(self.raw.len());
        let mut buffered: Option<Vec<T>> = None;

        let mut rows = self.raw.take_all();
        for (i, mut row) in rows.drain(..).enumerate().rev() {
            // Append lines left over from the previous row.
            if let Some(buffered) = buffered.take() {
                // Add a column for every cell added before the cursor, if it goes beyond the new
                // width it is then later reflown.
                let cursor_buffer_line = (self.lines - self.cursor.point.line - 1).0;
                if i == cursor_buffer_line {
                    self.cursor.point.col += buffered.len();
                }

                row.append_front(buffered);
            }

            loop {
                // Remove all cells which require reflowing.
                let mut wrapped = match row.shrink(cols) {
                    Some(wrapped) if reflow => wrapped,
                    _ => {
                        let cursor_buffer_line = (self.lines - self.cursor.point.line - 1).0;
                        if reflow && i == cursor_buffer_line && self.cursor.point.col > cols {
                            // If there are empty cells before the cursor, we assume it is explicit
                            // whitespace and need to wrap it like normal content.
                            Vec::new()
                        } else {
                            // Since it fits, just push the existing line without any reflow.
                            new_raw.push(row);
                            break;
                        }
                    },
                };

                // Insert spacer if a wide char would be wrapped into the last column.
                if row.len() >= cols.0 && row[cols - 1].flags().contains(Flags::WIDE_CHAR) {
                    wrapped.insert(0, row[cols - 1]);

                    let mut spacer = T::default();
                    spacer.flags_mut().insert(Flags::LEADING_WIDE_CHAR_SPACER);
                    row[cols - 1] = spacer;
                }

                // Remove wide char spacer before shrinking.
                let len = wrapped.len();
                if len > 0 && wrapped[len - 1].flags().contains(Flags::LEADING_WIDE_CHAR_SPACER) {
                    if len == 1 {
                        row[cols - 1].flags_mut().insert(Flags::WRAPLINE);
                        new_raw.push(row);
                        break;
                    } else {
                        // Remove the leading spacer from the end of the wrapped row.
                        wrapped[len - 2].flags_mut().insert(Flags::WRAPLINE);
                        wrapped.truncate(len - 1);
                    }
                }

                new_raw.push(row);

                // Set line as wrapped if cells got removed.
                if let Some(cell) = new_raw.last_mut().and_then(|r| r.last_mut()) {
                    cell.flags_mut().insert(Flags::WRAPLINE);
                }

                if wrapped
                    .last()
                    .map(|c| c.flags().contains(Flags::WRAPLINE) && i >= 1)
                    .unwrap_or(false)
                    && wrapped.len() < cols.0
                {
                    // Make sure previous wrap flag doesn't linger around.
                    if let Some(cell) = wrapped.last_mut() {
                        cell.flags_mut().remove(Flags::WRAPLINE);
                    }

                    // Add removed cells to start of next row.
                    buffered = Some(wrapped);
                    break;
                } else {
                    // Reflow the cursor if it is on this line beyond the width.
                    let cursor_buffer_line = (self.lines - self.cursor.point.line - 1).0;
                    if cursor_buffer_line == i && self.cursor.point.col >= cols {
                        // Since only a single new line is created, we subtract only `cols` from
                        // the cursor instead of reflowing it completely.
                        self.cursor.point.col -= cols;
                    }

                    // Make sure new row is at least as long as new width.
                    let occ = wrapped.len();
                    if occ < cols.0 {
                        wrapped.append(&mut vec![T::default(); cols.0 - occ]);
                    }
                    row = Row::from_vec(wrapped, occ);
                }
            }
        }

        // Reverse iterator and use it as the new grid storage.
        let mut reversed: Vec<Row<T>> = new_raw.drain(..).rev().collect();
        reversed.truncate(self.max_scroll_limit + self.lines.0);
        self.raw.replace_inner(reversed);

        // Reflow the primary cursor, or clamp it if reflow is disabled.
        if !reflow {
            self.cursor.point.col = min(self.cursor.point.col, cols - 1);
        } else if self.cursor.point.col == cols {
            self.cursor.input_needs_wrap = true;
            self.cursor.point.col -= 1;
        } else {
            self.cursor.point = self.cursor.point.add(cols, 0);
        }

        // Clamp the saved cursor to the grid.
        self.saved_cursor.point.col = min(self.saved_cursor.point.col, cols - 1);
    }
