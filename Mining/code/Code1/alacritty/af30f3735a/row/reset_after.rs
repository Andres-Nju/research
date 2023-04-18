    pub fn reset(&mut self, other: &T) {
        for item in &mut self.inner[..] {
            *item = *other;
        }
        self.occ = 0;
    }
