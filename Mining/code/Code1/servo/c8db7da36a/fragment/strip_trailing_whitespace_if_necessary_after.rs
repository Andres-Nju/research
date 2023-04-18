    pub fn strip_trailing_whitespace_if_necessary(&mut self) -> WhitespaceStrippingResult {
        if self.white_space().preserve_spaces() {
            return WhitespaceStrippingResult::RetainFragment
        }

        match self.specific {
            SpecificFragmentInfo::ScannedText(ref mut scanned_text_fragment_info) => {
                let mut trailing_whitespace_start_byte = 0;
                for (i, c) in scanned_text_fragment_info.text().char_indices().rev() {
                    if !util::str::char_is_whitespace(c) {
                        trailing_whitespace_start_byte = i + c.len_utf8();
                        break;
                    }
                }
                let whitespace_start = ByteIndex(trailing_whitespace_start_byte as isize);
                let whitespace_len = scanned_text_fragment_info.range.length() - whitespace_start;
                let mut whitespace_range = Range::new(whitespace_start, whitespace_len);
                whitespace_range.shift_by(scanned_text_fragment_info.range.begin());

                let text_bounds = scanned_text_fragment_info.run
                                                        .metrics_for_range(&whitespace_range)
                                                        .bounding_box;
                self.border_box.size.inline -= text_bounds.size.width;
                scanned_text_fragment_info.content_size.inline -= text_bounds.size.width;

                scanned_text_fragment_info.range.extend_by(-whitespace_len);
                WhitespaceStrippingResult::RetainFragment
            }
            SpecificFragmentInfo::UnscannedText(ref mut unscanned_text_fragment_info) => {
                let mut trailing_bidi_control_characters_to_retain = Vec::new();
                let (mut modified, mut last_character_index) = (true, 0);
                for (i, character) in unscanned_text_fragment_info.text.char_indices().rev() {
                    if gfx::text::util::is_bidi_control(character) {
                        trailing_bidi_control_characters_to_retain.push(character);
                        continue
                    }
                    if util::str::char_is_whitespace(character) {
                        modified = true;
                        continue
                    }
                    last_character_index = i + character.len_utf8();
                    break
                }
                if modified {
                    let mut text = unscanned_text_fragment_info.text.to_string();
                    text.truncate(last_character_index);
                    for character in trailing_bidi_control_characters_to_retain.iter().rev() {
                        text.push(*character);
                    }
                    unscanned_text_fragment_info.text = text.into_boxed_str();
                }

                WhitespaceStrippingResult::from_unscanned_text_fragment_info(
                    &unscanned_text_fragment_info)
            }
            _ => WhitespaceStrippingResult::RetainFragment,
        }
    }
