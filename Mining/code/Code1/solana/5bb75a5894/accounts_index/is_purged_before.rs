    pub fn is_purged(&self, fork: Fork) -> bool {
        !self.is_root(fork) && fork < self.last_root
    }
