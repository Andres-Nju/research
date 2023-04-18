    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.cursor_offset == self.inner.offset() && self.inner.column() == self.cursor.col {
                let selected = self
                    .selection
                    .as_ref()
                    .map(|range| range.contains(self.cursor.col, self.cursor.line))
                    .unwrap_or(false);

                // Handle cursor
                if let Some(cursor_key) = self.cursor_key.take() {
                    let cell = Indexed {
                        inner: self.grid[self.cursor],
                        column: self.cursor.col,
                        line: self.cursor.line,
                    };

                    let mut renderable_cell =
                        RenderableCell::new(self.config, self.colors, cell, selected);

                    renderable_cell.inner = RenderableCellContent::Cursor(cursor_key);

                    if let Some(color) = self.config.colors.cursor.cursor {
                        renderable_cell.fg = color;
                    }

                    return Some(renderable_cell);
                } else {
                    let mut cell =
                        RenderableCell::new(self.config, self.colors, self.inner.next()?, selected);

                    if self.cursor_style == CursorStyle::Block {
                        std::mem::swap(&mut cell.bg, &mut cell.fg);

                        if let Some(color) = self.config.colors.cursor.text {
                            cell.fg = color;
                        }
                    }

                    return Some(cell);
                }
            } else {
                let mut cell = self.inner.next()?;

                let selected = self
                    .selection
                    .as_ref()
                    .map(|range| range.contains(cell.column, cell.line))
                    .unwrap_or(false);

                // Underline URL highlights
                let index = Linear::new(self.grid.num_cols(), cell.column, cell.line);
                if self.url_highlight.as_ref().map(|range| range.contains_(index)).unwrap_or(false)
                {
                    cell.inner.flags.insert(Flags::UNDERLINE);
                }

                if !cell.is_empty() || selected {
                    return Some(RenderableCell::new(self.config, self.colors, cell, selected));
                }
            }
        }
    }
