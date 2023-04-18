    pub fn enter() -> Scope {
        IN_SCOPE.with(|slot| *slot.borrow_mut() = true);
        Scope { _hidden: () }
    }
