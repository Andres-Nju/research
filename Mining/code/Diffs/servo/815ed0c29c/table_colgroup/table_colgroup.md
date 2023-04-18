File_Code/servo/815ed0c29c/table_colgroup/table_colgroup_after.rs --- Rust
95     fn collect_stacking_contexts(&mut self, _: &mut DisplayListBuildState) {}                                                                             95     fn collect_stacking_contexts(&mut self, state: &mut DisplayListBuildState) {
                                                                                                                                                             96         self.base.stacking_context_id = state.current_stacking_context_id;
                                                                                                                                                             97         self.base.scroll_root_id = Some(state.current_scroll_root_id);

