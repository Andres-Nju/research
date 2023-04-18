    fn drop(&mut self) {
        IN_SCOPE.with(|slot| *slot.borrow_mut() = false);
    }
