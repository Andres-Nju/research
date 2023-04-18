    pub fn scroll_up(&mut self, region: &Range<Line>, positions: Line, template: &T) {
        let num_lines = self.num_lines().0;
        let num_cols = self.num_cols().0;

        if region.start == Line(0) {
            // Update display offset when not pinned to active area
            if self.display_offset != 0 {
                self.display_offset = min(self.display_offset + *positions, self.max_scroll_limit);
            }

            self.increase_scroll_limit(*positions, template);

            // Rotate the entire line buffer. If there's a scrolling region
            // active, the bottom lines are restored in the next step.
            self.raw.rotate(-(*positions as isize));
            self.selection = self
                .selection
                .take()
                .and_then(|s| s.rotate(num_lines, num_cols, region, *positions as isize));

            // This next loop swaps "fixed" lines outside of a scroll region
            // back into place after the rotation. The work is done in buffer-
            // space rather than terminal-space to avoid redundant
            // transformations.
            let fixed_lines = num_lines - *region.end;

            for i in 0..fixed_lines {
                self.raw.swap(i, i + *positions);
            }

            // Finally, reset recycled lines
            //
            // Recycled lines are just above the end of the scrolling region.
            for i in 0..*positions {
                self.raw[i + fixed_lines].reset(&template);
            }
        } else {
            // Rotate selection to track content
            self.selection = self
                .selection
                .take()
                .and_then(|s| s.rotate(num_lines, num_cols, region, *positions as isize));

            // Subregion rotation
            for line in IndexRange(region.start..(region.end - positions)) {
                self.raw.swap_lines(line, line + positions);
            }

            // Clear reused lines
            for line in IndexRange((region.end - positions)..region.end) {
                self.raw[line].reset(&template);
            }
        }
    }
