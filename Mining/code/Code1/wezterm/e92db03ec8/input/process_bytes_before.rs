    fn process_bytes<F: FnMut(InputEvent)>(&mut self, mut callback: F, maybe_more: bool) {
        while !self.buf.is_empty() {
            match self.state {
                InputState::Pasting(offset) => {
                    let end_paste = b"\x1b[201~";
                    if let Some(idx) = self.buf.find_subsequence(offset, end_paste) {
                        let pasted =
                            String::from_utf8_lossy(&self.buf.as_slice()[0..idx]).to_string();
                        self.buf.advance(pasted.len() + end_paste.len());
                        callback(InputEvent::Paste(pasted));
                        self.state = InputState::Normal;
                    } else {
                        self.state = InputState::Pasting(0);
                        return;
                    }
                }
                InputState::EscapeMaybeAlt | InputState::Normal => {
                    if self.state == InputState::Normal && self.buf.as_slice()[0] == b'\x1b' {
                        // This feels a bit gross because we have two different parsers at play
                        // here.  We want to re-use the escape sequence parser to crack the
                        // parameters out from things like mouse reports.  The keymap tree doesn't
                        // know how to grok this.
                        let mut parser = Parser::new();
                        if let Some((Action::CSI(CSI::Mouse(mouse)), len)) =
                            parser.parse_first(self.buf.as_slice())
                        {
                            self.buf.advance(len);

                            match mouse {
                                MouseReport::SGR1006 {
                                    x,
                                    y,
                                    button,
                                    modifiers,
                                } => {
                                    callback(InputEvent::Mouse(MouseEvent {
                                        x,
                                        y,
                                        mouse_buttons: button.into(),
                                        modifiers,
                                    }));
                                }
                                MouseReport::SGR1016 {
                                    x_pixels,
                                    y_pixels,
                                    button,
                                    modifiers,
                                } => {
                                    callback(InputEvent::PixelMouse(PixelMouseEvent {
                                        x_pixels: x_pixels,
                                        y_pixels: y_pixels,
                                        mouse_buttons: button.into(),
                                        modifiers,
                                    }));
                                }
                            }
                            continue;
                        }
                    }

                    match (self.key_map.lookup(self.buf.as_slice()), maybe_more) {
                        // If we got an unambiguous ESC and we have more data to
                        // follow, then this is likely the Meta version of the
                        // following keypress.  Buffer up the escape key and
                        // consume it from the input.  dispatch_callback() will
                        // emit either the ESC or the ALT modified following key.
                        (
                            Found::Exact(
                                len,
                                InputEvent::Key(KeyEvent {
                                    key: KeyCode::Escape,
                                    modifiers: Modifiers::NONE,
                                }),
                            ),
                            _,
                        ) if self.state == InputState::Normal && self.buf.len() > len => {
                            self.state = InputState::EscapeMaybeAlt;
                            self.buf.advance(len);
                        }
                        (Found::Exact(len, event), _) | (Found::Ambiguous(len, event), false) => {
                            self.dispatch_callback(&mut callback, event.clone());
                            self.buf.advance(len);
                        }
                        (Found::Ambiguous(_, _), true) | (Found::NeedData, true) => {
                            return;
                        }
                        (Found::None, _) | (Found::NeedData, false) => {
                            // No pre-defined key, so pull out a unicode character
                            if let Some((c, len)) = Self::decode_one_char(self.buf.as_slice()) {
                                self.buf.advance(len);
                                self.dispatch_callback(
                                    &mut callback,
                                    InputEvent::Key(KeyEvent {
                                        key: KeyCode::Char(c),
                                        modifiers: Modifiers::NONE,
                                    }),
                                );
                            } else {
                                // We need more data to recognize the input, so
                                // yield the remainder of the slice
                                return;
                            }
                        }
                    }
                }
            }
        }
    }
