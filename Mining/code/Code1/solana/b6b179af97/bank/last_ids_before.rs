    pub fn last_ids(&self) -> &RwLock<StatusDeque<Result<()>>> {
        &self.last_ids
    }
