File_Code/wezterm/e92db03ec8/input/input_after.rs --- Rust
   .                                                                                                                                                         1279                         // Advance our offset so that in the case where we receive a paste that
   .                                                                                                                                                         1280                         // is spread across N reads of size 8K, we don't need to search for the
   .                                                                                                                                                         1281                         // end marker in 8K, 16K, 24K etc. of text until the final buffer is received.
   .                                                                                                                                                         1282                         // Ensure that we use saturating math here for the case where the amount
   .                                                                                                                                                         1283                         // of buffered data after the begin paste is smaller than the end paste marker
   .                                                                                                                                                         1284                         // <https://github.com/wez/wezterm/pull/1832> 
1279                         self.state = InputState::Pasting(0);                                                                                            1285                         self.state = InputState::Pasting(self.buf.len().saturating_sub(end_paste.len()));
