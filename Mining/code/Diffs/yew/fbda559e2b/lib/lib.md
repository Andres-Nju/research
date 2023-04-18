File_Code/yew/fbda559e2b/lib/lib_after.rs --- 1/3 --- Rust
                                                                                                                                                           107                 self.state.clear_all_edit();

File_Code/yew/fbda559e2b/lib/lib_after.rs --- 2/3 --- Rust
232                        value=&entry.description                                                                                                          233                        value=&self.state.edit_value

File_Code/yew/fbda559e2b/lib/lib_after.rs --- 3/3 --- Rust
                                                                                                                                                             338     fn clear_all_edit(&mut self) {
                                                                                                                                                             339         for entry in self.entries.iter_mut() {
                                                                                                                                                             340             entry.editing = false;
                                                                                                                                                             341         }
                                                                                                                                                             342     }

