    fn osc_dispatch(&mut self, params: &[&[u8]]) {
        let writer = &mut self.writer;

        fn unhandled(params: &[&[u8]]) {
            let mut buf = String::new();
            for items in params {
                buf.push_str("[");
                for item in *items {
                    buf.push_str(&format!("{:?},", *item as char));
                }
                buf.push_str("],");
            }
            debug!("[unhandled osc_dispatch]: [{}] at line {}", &buf, line!());
        }

        if params.is_empty() || params[0].is_empty() {
            return;
        }

        match params[0] {
            // Set window title
            b"0" | b"2" => {
                if params.len() >= 2 {
                    let title = params[1..]
                        .iter()
                        .flat_map(|x| str::from_utf8(x))
                        .collect::<Vec<&str>>()
                        .join(";");
                    self.handler.set_title(&title);
                    return;
                }
                unhandled(params);
            },

            // Set icon name
            // This is ignored, since alacritty has no concept of tabs
            b"1" => (),

            // Set color index
            b"4" => {
                if params.len() > 1 && params.len() % 2 != 0 {
                    for chunk in params[1..].chunks(2) {
                        let index = parse_number(chunk[0]);
                        let color = xparse_color(chunk[1]);
                        if let (Some(i), Some(c)) = (index, color) {
                            self.handler.set_color(i as usize, c);
                            return;
                        }
                    }
                }
                unhandled(params);
            },

            // Get/set Foreground, Background, Cursor colors
            b"10" | b"11" | b"12" => {
                if params.len() >= 2 {
                    if let Some(mut dynamic_code) = parse_number(params[0]) {
                        for param in &params[1..] {
                            // 10 is the first dynamic color, also the foreground
                            let offset = dynamic_code as usize - 10;
                            let index = NamedColor::Foreground as usize + offset;

                            // End of setting dynamic colors
                            if index > NamedColor::Cursor as usize {
                                unhandled(params);
                                break;
                            }

                            if let Some(color) = xparse_color(param) {
                                self.handler.set_color(index, color);
                            } else if param == b"?" {
                                self.handler.dynamic_color_sequence(writer, dynamic_code, index);
                            } else {
                                unhandled(params);
                            }
                            dynamic_code += 1;
                        }
                        return;
                    }
                }
                unhandled(params);
            },

            // Set cursor style
            b"50" => {
                if params.len() >= 2
                    && params[1].len() >= 13
                    && params[1][0..12] == *b"CursorShape="
                {
                    let style = match params[1][12] as char {
                        '0' => CursorStyle::Block,
                        '1' => CursorStyle::Beam,
                        '2' => CursorStyle::Underline,
                        _ => return unhandled(params),
                    };
                    self.handler.set_cursor_style(Some(style));
                    return;
                }
                unhandled(params);
            },

            // Set clipboard
            b"52" => {
                if params.len() < 3 || params[1].is_empty() {
                    return unhandled(params);
                }

                match params[2] {
                    b"?" => self.handler.write_clipboard(params[1][0], writer),
                    base64 => self.handler.set_clipboard(params[1][0], base64),
                }
            },

            // Reset color index
            b"104" => {
                // Reset all color indexes when no parameters are given
                if params.len() == 1 {
                    for i in 0..256 {
                        self.handler.reset_color(i);
                    }
                    return;
                }

                // Reset color indexes given as parameters
                for param in &params[1..] {
                    match parse_number(param) {
                        Some(index) => self.handler.reset_color(index as usize),
                        None => unhandled(params),
                    }
                }
            },

            // Reset foreground color
            b"110" => self.handler.reset_color(NamedColor::Foreground as usize),

            // Reset background color
            b"111" => self.handler.reset_color(NamedColor::Background as usize),

            // Reset text cursor color
            b"112" => self.handler.reset_color(NamedColor::Cursor as usize),

            _ => unhandled(params),
        }
    }
