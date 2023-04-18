    pub fn reset(&mut self, other: &T) {
        for item in &mut self.inner[..self.occ] {
            *item = *other;
        }
        self.occ = 0;
    }
