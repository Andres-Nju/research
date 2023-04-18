File_Code/alacritty/b3afb97fcd/display/display_after.rs --- Text (2 errors, exceeded DFT_PARSE_ERROR_LIMIT)
335         if resize_pending.message_buffer.is_some() {                                                                                                     335         if let Some(message) = message_buffer.message() {
336             let lines =                                                                                                                                  336             let lines = message.text(&self.size_info).len();
337                 message_buffer.message().map(|m| m.text(&self.size_info).len()).unwrap_or(0);                                                                

