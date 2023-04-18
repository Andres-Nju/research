    pub fn stolen(&self) -> bool {
        self.value.borrow().is_none()
    }
