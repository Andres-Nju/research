File_Code/rust/211365d68c/mod/mod_after.rs --- Text (4 errors, exceeded DFT_PARSE_ERROR_LIMIT)
4862     /// Given a function block's `NodeId`, return its `FnDecl` , `None` otherwise.                                                                      4862     /// Given a function block's `NodeId`, return its `FnDecl` if it exists, or `None` otherwise.
4863     fn get_parent_fn_decl(&self, blk_id: ast::NodeId) -> Option<(hir::FnDecl, ast::Ident)> {                                                            4863     fn get_parent_fn_decl(&self, blk_id: ast::NodeId) -> Option<(hir::FnDecl, ast::Ident)> {
4864         let parent = self.tcx.hir().get(self.tcx.hir().get_parent(blk_id));                                                                             4864         let parent = self.tcx.hir().get(self.tcx.hir().get_parent(blk_id));
4865         self.get_node_fn_decl(parent).map(|(fn_decl, ident , _)| (fn_decl, ident))                                                                      4865         self.get_node_fn_decl(parent).map(|(fn_decl, ident, _)| (fn_decl, ident))
4866     }                                                                                                                                                   4866     }
4867                                                                                                                                                         4867 
4868     /// Given a function `Node`, return its `FnDecl` , `None` otherwise.                                                                                4868     /// Given a function `Node`, return its `FnDecl` if it exists, or `None` otherwise.

