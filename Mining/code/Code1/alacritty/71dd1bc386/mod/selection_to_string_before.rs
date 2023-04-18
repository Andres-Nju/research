    pub fn selection_to_string(&self) -> Option<String> {
        let selection = self.grid.selection.clone()?;
        let SelectionRange { start, end, is_block } = selection.to_range(self)?;

        let mut res = String::new();

        if is_block {
            for line in (end.line + 1..=start.line).rev() {
                res += &(self.line_to_string(line, start.col..end.col, start.col.0 != 0) + "\n");
            }
            res += &self.line_to_string(end.line, start.col..end.col, true);
        } else {
            res = self.bounds_to_string(start, end);
        }

        Some(res)
    }
