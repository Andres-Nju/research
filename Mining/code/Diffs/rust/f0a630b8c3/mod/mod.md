File_Code/rust/f0a630b8c3/mod/mod_after.rs --- Rust
242     fn visit_foreign_item(&mut self, item: &'tcx hir::ForeignItem) {                                                                                       
243         self.calculate_node_id(item.id, |v| v.visit_foreign_item(item));                                                                                   
244         visit::walk_foreign_item(self, item);                                                                                                              
245     }                                                                                                                                                      

