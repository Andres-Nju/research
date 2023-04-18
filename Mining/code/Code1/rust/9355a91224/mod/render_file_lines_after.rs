    fn render_file_lines(&self, codemap: &Rc<CodeMap>) -> Vec<RenderedLine> {
        let old_school = check_old_skool();

        // As a first step, we elide any instance of more than one
        // continuous unannotated line.

        let mut lines_iter = self.lines.iter();
        let mut output = vec![];

        // First insert the name of the file.
        if !old_school {
            match self.primary_span {
                Some(span) => {
                    let lo = codemap.lookup_char_pos(span.lo);
                    output.push(RenderedLine {
                        text: vec![StyledString {
                            text: lo.file.name.clone(),
                            style: Style::FileNameStyle,
                        }, StyledString {
                            text: format!(":{}:{}", lo.line, lo.col.0 + 1),
                            style: Style::LineAndColumn,
                        }],
                        kind: RenderedLineKind::PrimaryFileName,
                    });
                }
                None => {
                    output.push(RenderedLine {
                        text: vec![StyledString {
                            text: self.file.name.clone(),
                            style: Style::FileNameStyle,
                        }],
                        kind: RenderedLineKind::OtherFileName,
                    });
                }
            }
        }

        let mut next_line = lines_iter.next();
        while next_line.is_some() {
            // Consume lines with annotations.
            while let Some(line) = next_line {
                if line.annotations.is_empty() { break; }

                let mut rendered_lines = self.render_line(line);
                assert!(!rendered_lines.is_empty());
                if old_school {
                    match self.primary_span {
                        Some(span) => {
                            let lo = codemap.lookup_char_pos(span.lo);
                            rendered_lines[0].text.insert(0, StyledString {
                                text: format!(":{} ", lo.line),
                                style: Style::LineAndColumn,
                            });
                            rendered_lines[0].text.insert(0, StyledString {
                                text: lo.file.name.clone(),
                                style: Style::FileNameStyle,
                            });
                            let gap_amount =
                                rendered_lines[0].text[0].text.len() +
                                rendered_lines[0].text[1].text.len();
                            assert!(rendered_lines.len() >= 2,
                                    "no annotations resulted from: {:?}",
                                    line);
                            for i in 1..rendered_lines.len() {
                                rendered_lines[i].text.insert(0, StyledString {
                                    text: vec![" "; gap_amount].join(""),
                                    style: Style::NoStyle
                                });
                            }
                        }
                        _ =>()
                    }
                }
                output.append(&mut rendered_lines);
                next_line = lines_iter.next();
            }

            // Emit lines without annotations, but only if they are
            // followed by a line with an annotation.
            let unannotated_line = next_line;
            let mut unannotated_lines = 0;
            while let Some(line) = next_line {
                if !line.annotations.is_empty() { break; }
                unannotated_lines += 1;
                next_line = lines_iter.next();
            }
            if unannotated_lines > 1 {
                output.push(RenderedLine::from((String::new(),
                                                Style::NoStyle,
                                                RenderedLineKind::Elision)));
            } else if let Some(line) = unannotated_line {
                output.append(&mut self.render_line(line));
            }
        }

        output
    }
