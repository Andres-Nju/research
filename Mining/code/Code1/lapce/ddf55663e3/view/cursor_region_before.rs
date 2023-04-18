    fn cursor_region(data: &LapceEditorBufferData, text: &mut PietText) -> Rect {
        let offset = data.editor.cursor.offset();
        let (line, col) = data.doc.buffer().offset_to_line_col(offset);
        let inlay_hints = data.doc.line_phantom_text(&data.config, line);
        let col = inlay_hints.col_at(col);

        let width = data.config.editor_char_width(text);
        let cursor_x = data
            .doc
            .line_point_of_line_col(
                text,
                line,
                col,
                data.config.editor.font_size,
                &data.config,
            )
            .x;
        let line_height = data.config.editor.line_height() as f64;

        let y = if data.editor.is_code_lens() {
            let empty_vec = Vec::new();
            let normal_lines = data
                .doc
                .syntax()
                .map(|s| &s.normal_lines)
                .unwrap_or(&empty_vec);

            let mut y = 0.0;
            let mut current_line = 0;
            let mut normal_lines = normal_lines.iter();
            loop {
                match normal_lines.next() {
                    Some(next_normal_line) => {
                        let next_normal_line = *next_normal_line;
                        if next_normal_line < line {
                            let chunk_height = data.config.editor.code_lens_font_size
                                as f64
                                * (next_normal_line - current_line) as f64
                                + line_height;
                            y += chunk_height;
                            current_line = next_normal_line + 1;
                            continue;
                        };
                        y += (line - current_line) as f64
                            * data.config.editor.code_lens_font_size as f64;
                        break;
                    }
                    None => {
                        y += (line - current_line) as f64
                            * data.config.editor.code_lens_font_size as f64;
                        break;
                    }
                }
            }
            y
        } else {
            let line = if let EditorView::Diff(version) = &data.editor.view {
                data.doc.history_visual_line(version, line)
            } else {
                line
            };
            line as f64 * line_height
        };

        let surrounding_lines_height =
            (data.config.editor.cursor_surrounding_lines as f64 * line_height)
                .min(data.editor.size.borrow().height / 2.);

        Rect::ZERO
            .with_size(Size::new(width, line_height))
            .with_origin(Point::new(cursor_x, y))
            .inflate(width, surrounding_lines_height)
    }
