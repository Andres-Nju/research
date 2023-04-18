    fn osc_dispatch(&mut self, params: &[&[u8]]) {
        fn unhandled(params: &[&[u8]]) {
            let mut buf = String::new();
            for items in params {
                buf.push_str("[");
                for item in *items {
                    buf.push_str(&format!("{:?},", *item as char));
                }
                buf.push_str("],");
            }
            warn!("[unhandled osc_dispatch]: [{}] at line {}", &buf, line!());
        }

        if params.is_empty() || params[0].is_empty() {
            return;
        }

        match params[0] {
            // Set window title
            b"0" | b"2" => {
                if params.len() >= 2 {
                    if let Ok(utf8_title) = str::from_utf8(params[1]) {
                        self.handler.set_title(utf8_title);
                        return;
                    }
                }
                unhandled(params);
            },

            // Set icon name
            // This is ignored, since alacritty has no concept of tabs
            b"1" => return,

            // Set color index
            b"4" => {
                if params.len() > 1 && params.len() % 2 != 0 {
                    for chunk in params[1..].chunks(2) {
                        let index = parse_number(chunk[0]);
                        let color = parse_rgb_color(chunk[0]);
                        if let (Some(i), Some(c)) = (index, color) {
                            self.handler.set_color(i as usize, c);
                            return;
                        }
                    }
                }
                unhandled(params);
            }

            // Set foreground color
            b"10" => {
                if params.len() >= 2 {
                    if let Some(color) = parse_rgb_color(params[1]) {
                        self.handler.set_color(NamedColor::Foreground as usize, color);
                        return;
                    }
                }
                unhandled(params);
            }

            // Set background color
            b"11" => {
                if params.len() >= 2 {
                    if let Some(color) = parse_rgb_color(params[1]) {
                        self.handler.set_color(NamedColor::Background as usize, color);
                        return;
                    }
                }
                unhandled(params);
            }

            // Set text cursor color
            b"12" => {
                if params.len() >= 2 {
                    if let Some(color) = parse_rgb_color(params[1]) {
                        self.handler.set_color(NamedColor::Cursor as usize, color);
                        return;
                    }
                }
                unhandled(params);
            }

            // Set clipboard
            b"52" => {
                if params.len() < 3 {
                    return unhandled(params);
                }

                match params[2] {
                    b"?" => unhandled(params),
                    selection => {
                        if let Ok(string) = base64::decode(selection) {
                            if let Ok(utf8_string) = str::from_utf8(&string) {
                                self.handler.set_clipboard(utf8_string);
                            }
                        }
                    }
                }
            }

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
            }

            // Reset foreground color
            b"110" => self.handler.reset_color(NamedColor::Foreground as usize),

            // Reset background color
            b"111" => self.handler.reset_color(NamedColor::Background as usize),

            // Reset text cursor color
            b"112" => self.handler.reset_color(NamedColor::Cursor as usize),

            _ => unhandled(params),
        }
    }
