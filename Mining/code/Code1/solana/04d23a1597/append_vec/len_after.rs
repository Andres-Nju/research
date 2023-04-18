    pub fn len(&self) -> usize {
        self.current_len.load(Ordering::Acquire)
    }
