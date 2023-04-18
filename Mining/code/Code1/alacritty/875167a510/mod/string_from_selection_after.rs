    pub fn string_from_selection(&self, span: &Span) -> String {
        /// Need a generic push() for the Append trait
        trait PushChar {
            fn push_char(&mut self, c: char);
            fn maybe_newline(&mut self, grid: &Grid<Cell>, line: Line, ending: Column) {
                if ending != Column(0) && !grid[line][ending - 1].flags.contains(cell::WRAPLINE) {
                    self.push_char('\n');
                }
            }
        }

        impl PushChar for String {
            #[inline]
            fn push_char(&mut self, c: char) {
                self.push(c);
            }
        }
        trait Append<T> : PushChar {
            fn append(&mut self, grid: &Grid<Cell>, line: Line, cols: T) -> Option<Range<Column>>;
        }

        use std::ops::{Range, RangeTo, RangeFrom, RangeFull};

        impl Append<Range<Column>> for String {
            fn append(
                &mut self,
                grid: &Grid<Cell>,
                line: Line,
                cols: Range<Column>
            ) -> Option<Range<Column>> {
                let line = &grid[line];
                let line_length = line.line_length();
                let line_end = min(line_length, cols.end + 1);

                if cols.start >= line_end {
                    None
                } else {
                    for cell in &line[cols.start..line_end] {
                        self.push(cell.c);
                    }

                    Some(cols.start..line_end)
                }
            }
        }

        impl Append<RangeTo<Column>> for String {
            #[inline]
            fn append(&mut self, grid: &Grid<Cell>, line: Line, cols: RangeTo<Column>) -> Option<Range<Column>> {
                self.append(grid, line, Column(0)..cols.end)
            }
        }

        impl Append<RangeFrom<Column>> for String {
            #[inline]
            fn append(
                &mut self,
                grid: &Grid<Cell>,
                line: Line,
                cols: RangeFrom<Column>
            ) -> Option<Range<Column>> {
                let range = self.append(grid, line, cols.start..Column(usize::max_value() - 1));
                range.as_ref()
                    .map(|range| self.maybe_newline(grid, line, range.end));
                range
            }
        }

        impl Append<RangeFull> for String {
            #[inline]
            fn append(
                &mut self,
                grid: &Grid<Cell>,
                line: Line,
                _: RangeFull
            ) -> Option<Range<Column>> {
                let range = self.append(grid, line, Column(0)..Column(usize::max_value() - 1));
                range.as_ref()
                    .map(|range| self.maybe_newline(grid, line, range.end));
                range
            }
        }

        let mut res = String::new();

        let (start, end) = span.to_locations(self.grid.num_cols());
        let line_count = end.line - start.line;

        match line_count {
            // Selection within single line
            Line(0) => {
                res.append(&self.grid, start.line, start.col..end.col);
            },

            // Selection ends on line following start
            Line(1) => {
                // Starting line
                res.append(&self.grid, start.line, start.col..);

                // Ending line
                res.append(&self.grid, end.line, ..end.col);
            },

            // Multi line selection
            _ => {
                // Starting line
                res.append(&self.grid, start.line, start.col..);

                let middle_range = IndexRange::from((start.line + 1)..(end.line));
                for line in middle_range {
                    res.append(&self.grid, line, ..);
                }

                // Ending line
                res.append(&self.grid, end.line, ..end.col);
            }
        }

        res
    }

    /// Convert the given pixel values to a grid coordinate
    ///
    /// The mouse coordinates are expected to be relative to the top left. The
    /// line and column returned are also relative to the top left.
    ///
    /// Returns None if the coordinates are outside the screen
    pub fn pixels_to_coords(&self, x: usize, y: usize) -> Option<Point> {
        self.size_info().pixels_to_coords(x, y)
    }

    /// Access to the raw grid data structure
    ///
    /// This is a bit of a hack; when the window is closed, the event processor
    /// serializes the grid state to a file.
    pub fn grid(&self) -> &Grid<Cell> {
        &self.grid
    }

    /// Iterate over the *renderable* cells in the terminal
    ///
    /// A renderable cell is any cell which has content other than the default
    /// background color.  Cells with an alternate background color are
    /// considered renderable as are cells with any text content.
    pub fn renderable_cells(&mut self, selection: &Selection) -> RenderableCellsIter {
        RenderableCellsIter::new(
            &mut self.grid,
            &self.cursor.point,
            self.mode,
            selection,
            self.custom_cursor_colors
        )
    }

    /// Resize terminal to new dimensions
    pub fn resize(&mut self, width: f32, height: f32) {
        let size = SizeInfo {
            width: width,
            height: height,
            cell_width: self.size_info.cell_width,
            cell_height: self.size_info.cell_height,
        };

        let old_cols = self.size_info.cols();
        let old_lines = self.size_info.lines();
        let mut num_cols = size.cols();
        let mut num_lines = size.lines();

        self.size_info = size;

        if old_cols == num_cols && old_lines == num_lines {
            return;
        }

        // Should not allow less than 1 col, causes all sorts of checks to be required.
        if num_cols <= Column(1) {
            num_cols = Column(2);
        }

        // Should not allow less than 1 line, causes all sorts of checks to be required.
        if num_lines <= Line(1) {
            num_lines = Line(2);
        }

        // Scroll up to keep cursor and as much context as possible in grid.
        // This only runs when the lines decreases.
        self.scroll_region = Line(0)..self.grid.num_lines();

        // Scroll up to keep cursor in terminal
        if self.cursor.point.line >= num_lines {
            let lines = self.cursor.point.line - num_lines + 1;
            self.scroll_up(lines);
            self.cursor.point.line -= lines;
        }

        println!("num_cols, num_lines = {}, {}", num_cols, num_lines);

        // Resize grids to new size
        let template = self.cursor.template;
        self.grid.resize(num_lines, num_cols, &template);
        self.alt_grid.resize(num_lines, num_cols, &template);

        // Ensure cursor is in-bounds
        self.cursor.point.line = limit(self.cursor.point.line, Line(0), num_lines - 1);
        self.cursor.point.col = limit(self.cursor.point.col, Column(0), num_cols - 1);

        // Recreate tabs list
        self.tabs = IndexRange::from(Column(0)..self.grid.num_cols())
            .map(|i| (*i as usize) % TAB_SPACES == 0)
            .collect::<Vec<bool>>();

        self.tabs[0] = false;

        if num_lines > old_lines {
            // Make sure bottom of terminal is clear
            let template = self.empty_cell;
            self.grid.clear_region((self.cursor.point.line + 1).., |c| c.reset(&template));
            self.alt_grid.clear_region((self.cursor.point.line + 1).., |c| c.reset(&template));
        }

        // Reset scrolling region to new size
        self.scroll_region = Line(0)..self.grid.num_lines();
    }

    #[inline]
    pub fn size_info(&self) -> &SizeInfo {
        &self.size_info
    }

    #[inline]
    pub fn mode(&self) -> &TermMode {
        &self.mode
    }

    pub fn swap_alt(&mut self) {
        if self.alt {
            let template = self.empty_cell;
            self.grid.clear(|c| c.reset(&template));
        }

        self.alt = !self.alt;
        ::std::mem::swap(&mut self.grid, &mut self.alt_grid);
    }

    /// Scroll screen down
    ///
    /// Text moves down; clear at bottom
    /// Expects origin to be in scroll range.
    #[inline]
    fn scroll_down_relative(&mut self, origin: Line, lines: Line) {
        trace!("scroll_down: {}", lines);

        // Copy of cell template; can't have it borrowed when calling clear/scroll
        let template = self.empty_cell;

        // Clear the entire region if lines is going to be greater than the region.
        // This also ensures all the math below this if statement is sane.
        if lines > self.scroll_region.end - origin {
            self.grid.clear_region(origin..self.scroll_region.end, |c| c.reset(&template));
            return;
        }

        // Clear `lines` lines at bottom of area
        {
            let end = self.scroll_region.end;
            let start = end - lines;
            self.grid.clear_region(start..end, |c| c.reset(&template));
        }

        // Scroll between origin and bottom
        {
            let end = self.scroll_region.end;
            let start = origin + lines;
            self.grid.scroll_down(start..end, lines);
        }
    }

    /// Scroll screen up
    ///
    /// Text moves up; clear at top
    /// Expects origin to be in scroll range.
    #[inline]
    fn scroll_up_relative(&mut self, origin: Line, lines: Line) {
        trace!("scroll_up: {}", lines);

        // Copy of cell template; can't have it borrowed when calling clear/scroll
        let template = self.empty_cell;

        // Clear the entire region if lines is going to be greater than the region.
        // This also ensures all the math below this if statement is sane.
        if lines > self.scroll_region.end - origin {
            self.grid.clear_region(origin..self.scroll_region.end, |c| c.reset(&template));
            return;
        }

        // Clear `lines` lines starting from origin to origin + lines
        {
            let end = origin + lines;
            self.grid.clear_region(origin..end, |c| c.reset(&template));
        }

        // Scroll from origin to bottom less number of lines
        {
            let end = self.scroll_region.end - lines;
            self.grid.scroll_up(origin..end, lines);
        }
    }
}
