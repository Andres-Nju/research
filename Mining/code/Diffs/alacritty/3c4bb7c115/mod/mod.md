File_Code/alacritty/3c4bb7c115/mod/mod_after.rs --- Rust
  .                                                                                                                                                          166         self.max_scroll_limit = history_size;
166         self.scroll_limit = min(self.scroll_limit, history_size);                                                                                        167         self.scroll_limit = min(self.scroll_limit, history_size);
                                                                                                                                                             168         self.display_offset = min(self.display_offset, self.scroll_limit);

