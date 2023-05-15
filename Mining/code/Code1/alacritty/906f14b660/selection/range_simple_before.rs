    fn range_simple(
        &self,
        mut start: Anchor,
        mut end: Anchor,
        num_cols: Column,
    ) -> Option<SelectionRange> {
        if self.is_empty() {
            return None;
        }

        // Remove last cell if selection ends to the left of a cell
        if end.side == Side::Left && start.point != end.point {
            // Special case when selection ends to left of first cell
            if end.point.col == Column(0) {
                end.point.col = num_cols;
                end.point.line += 1;
            } else {
                end.point.col -= 1;
            }
        }

        // Remove first cell if selection starts at the right of a cell
        if start.side == Side::Right && start.point != end.point {
            start.point.col += 1;
        }

        Some(SelectionRange { start: start.point, end: end.point, is_block: false })
    }