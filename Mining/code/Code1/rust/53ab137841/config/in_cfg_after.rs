    fn in_cfg(&mut self, attrs: &[ast::Attribute]) -> bool;

    // Update a node before checking if it is in this configuration (used to implement `cfg_attr`).
    fn process_attrs<T: HasAttrs>(&mut self, node: T) -> T { node }
