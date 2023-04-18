File_Code/rust/18d5f82148/rvalue_promotion/rvalue_promotion_after.rs --- Rust
  .                                                                                                                                                          251         let tables = self.tables;
251         euv::ExprUseVisitor::new(self, tcx, param_env, &region_scope_tree, self.tables, None)                                                            252         euv::ExprUseVisitor::new(self, tcx, param_env, &region_scope_tree, tables, None)

