    fn in_cfg(&mut self, attrs: &[ast::Attribute]) -> bool;
    fn process_attrs<T: HasAttrs>(&mut self, node: T) -> T { node }
