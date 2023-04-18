    pub fn swap_alt(&mut self) {
        self.alt = !self.alt;
        ::std::mem::swap(&mut self.grid, &mut self.alt_grid);

        if self.alt {
            let template = self.empty_cell;
            self.grid.clear(|c| c.reset(&template));
        }
    }
