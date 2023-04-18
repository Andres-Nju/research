File_Code/rust/9991fdf17a/dump_visitor/dump_visitor_after.rs --- Rust
   .                                                                                                                                                         1256         // The access is calculated using the current tree ID, but with the root tree's visibility
   .                                                                                                                                                         1257         // (since nested trees don't have their own visibility).
1255         let access = access_from!(self.save_ctxt, root_item);                                                                                           1258         let access = Access {
                                                                                                                                                             1259             public: root_item.vis == ast::Visibility::Public,
                                                                                                                                                             1260             reachable: self.save_ctxt.analysis.access_levels.is_reachable(id),
                                                                                                                                                             1261         };

