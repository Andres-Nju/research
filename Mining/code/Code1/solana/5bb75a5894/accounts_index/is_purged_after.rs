    pub fn is_purged(&self, fork: Fork) -> bool {
        fork < self.last_root
    }
