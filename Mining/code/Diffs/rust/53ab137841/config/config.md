File_Code/rust/53ab137841/config/config_after.rs --- Rust
 .                                                                                                                                                           22     // Check if a node with the given attributes is in this configuration.
22     fn in_cfg(&mut self, attrs: &[ast::Attribute]) -> bool;                                                                                               23     fn in_cfg(&mut self, attrs: &[ast::Attribute]) -> bool;
..                                                                                                                                                           24 
..                                                                                                                                                           25     // Update a node before checking if it is in this configuration (used to implement `cfg_attr`).
23     fn process_attrs<T: HasAttrs>(&mut self, node: T) -> T { node }                                                                                       26     fn process_attrs<T: HasAttrs>(&mut self, node: T) -> T { node }
..                                                                                                                                                           27 
..                                                                                                                                                           28     // Visit attributes on expression and statements (but not attributes on items in blocks).
24     fn visit_stmt_or_expr_attrs(&mut self, _attrs: &[ast::Attribute]) {}                                                                                  29     fn visit_stmt_or_expr_attrs(&mut self, _attrs: &[ast::Attribute]) {}
                                                                                                                                                             30 
                                                                                                                                                             31     // Visit unremovable (non-optional) expressions -- c.f. `fold_expr` vs `fold_opt_expr`.

