File_Code/alacritty/bb4fddd593/event_loop/event_loop_after.rs --- Rust
345                                 if !self.hold {                                                                                                          345                                 if self.hold {
                                                                                                                                                             346                                     // With hold enabled, make sure the PTY is drained.
                                                                                                                                                             347                                     let _ = self.pty_read(&mut state, &mut buf, pipe.as_mut());
                                                                                                                                                             348                                 } else {
                                                                                                                                                             349                                     // Without hold, shutdown the terminal.

