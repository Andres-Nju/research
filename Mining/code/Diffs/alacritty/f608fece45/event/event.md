File_Code/alacritty/f608fece45/event/event_after.rs --- Rust
                                                                                                                                                           405             // Prevent previous search selections from sticking around when not in vi mode.
                                                                                                                                                           406             if !self.terminal.mode().contains(TermMode::VI) {
                                                                                                                                                           407                 self.terminal.selection = None;
                                                                                                                                                           408             }

