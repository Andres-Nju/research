File_Code/rust/5d3be12a4d/dump_visitor/dump_visitor_after.rs --- 1/2 --- Rust
1418                 visit::walk_expr(self, subexpression);                                                                                                  1418                 self.visit_expr(subexpression);

File_Code/rust/5d3be12a4d/dump_visitor/dump_visitor_after.rs --- 2/2 --- Rust
1424                 visit::walk_expr(self, subexpression);                                                                                                  1424                 self.visit_expr(subexpression);
1425                 visit::walk_block(self, block);                                                                                                         1425                 visit::walk_block(self, block);
1426                 opt_else.as_ref().map(|el| visit::walk_expr(self, el));                                                                                 1426                 opt_else.as_ref().map(|el| self.visit_expr(el));

