File_Code/alacritty/c10888b0f0/event/event_after.rs --- 1/2 --- Rust
773             if self.search_state.dfas().is_some() {                                                                                                      773             if self.search_state.dfas.take().is_some() {

File_Code/alacritty/c10888b0f0/event/event_after.rs --- 2/2 --- Rust
...                                                                                                                                                          784         if self.search_active() {
784         self.cancel_search();                                                                                                                            785             self.cancel_search();
                                                                                                                                                             786         }

