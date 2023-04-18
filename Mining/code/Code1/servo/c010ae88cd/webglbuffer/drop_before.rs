    fn drop(&mut self) {
        self.mark_for_deletion();
        assert!(self.is_deleted());
    }
