File_Code/alacritty/b8e3050104/event/event_after.rs --- Rust
                                                                                                                                                          1007         // Update display if padding options were changed.
                                                                                                                                                          1008         let window_config = &processor.ctx.config.ui_config.window;
                                                                                                                                                          1009         if window_config.padding != config.ui_config.window.padding
                                                                                                                                                          1010             || window_config.dynamic_padding != config.ui_config.window.dynamic_padding
                                                                                                                                                          1011         {
                                                                                                                                                          1012             processor.ctx.display_update_pending.dirty = true;
                                                                                                                                                          1013         }

