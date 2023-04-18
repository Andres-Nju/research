    fn emit_message_default(&mut self,
                            msp: &MultiSpan,
                            msg: &Vec<(String, Style)>,
                            code: &Option<DiagnosticId>,
                            level: &Level,
                            max_line_num_len: usize,
                            is_secondary: bool)
                            -> io::Result<()> {
        let mut buffer = StyledBuffer::new();

        if msp.primary_spans().is_empty() && msp.span_labels().is_empty() && is_secondary
           && !self.short_message {
            // This is a secondary message with no span info
            for _ in 0..max_line_num_len {
                buffer.prepend(0, " ", Style::NoStyle);
            }
            draw_note_separator(&mut buffer, 0, max_line_num_len + 1);
            buffer.append(0, &level.to_string(), Style::HeaderMsg);
            buffer.append(0, ": ", Style::NoStyle);
            self.msg_to_buffer(&mut buffer, msg, max_line_num_len, "note", None);
        } else {
            buffer.append(0, &level.to_string(), Style::Level(level.clone()));
            // only render error codes, not lint codes
            if let Some(DiagnosticId::Error(ref code)) = *code {
                buffer.append(0, "[", Style::Level(level.clone()));
                buffer.append(0, &code, Style::Level(level.clone()));
                buffer.append(0, "]", Style::Level(level.clone()));
            }
            buffer.append(0, ": ", Style::HeaderMsg);
            for &(ref text, _) in msg.iter() {
                buffer.append(0, text, Style::HeaderMsg);
            }
        }

        // Preprocess all the annotations so that they are grouped by file and by line number
        // This helps us quickly iterate over the whole message (including secondary file spans)
        let mut annotated_files = self.preprocess_annotations(msp);

        // Make sure our primary file comes first
        let (primary_lo, cm) = if let (Some(cm), Some(ref primary_span)) =
            (self.cm.as_ref(), msp.primary_span().as_ref()) {
            if primary_span != &&DUMMY_SP {
                (cm.lookup_char_pos(primary_span.lo()), cm)
            } else {
                emit_to_destination(&buffer.render(), level, &mut self.dst, self.short_message)?;
                return Ok(());
            }
        } else {
            // If we don't have span information, emit and exit
            emit_to_destination(&buffer.render(), level, &mut self.dst, self.short_message)?;
            return Ok(());
        };
        if let Ok(pos) =
            annotated_files.binary_search_by(|x| x.file.name.cmp(&primary_lo.file.name)) {
            annotated_files.swap(0, pos);
        }

        // Print out the annotate source lines that correspond with the error
        for annotated_file in annotated_files {
            // we can't annotate anything if the source is unavailable.
            if !cm.ensure_filemap_source_present(annotated_file.file.clone()) {
                continue;
            }

            // print out the span location and spacer before we print the annotated source
            // to do this, we need to know if this span will be primary
            let is_primary = primary_lo.file.name == annotated_file.file.name;
            if is_primary {
                let loc = primary_lo.clone();
                if !self.short_message {
                    // remember where we are in the output buffer for easy reference
                    let buffer_msg_line_offset = buffer.num_lines();

                    buffer.prepend(buffer_msg_line_offset, "--> ", Style::LineNumber);
                    buffer.append(buffer_msg_line_offset,
                                  &format!("{}:{}:{}",
                                           loc.file.name,
                                           cm.doctest_offset_line(loc.line),
                                           loc.col.0 + 1),
                                  Style::LineAndColumn);
                    for _ in 0..max_line_num_len {
                        buffer.prepend(buffer_msg_line_offset, " ", Style::NoStyle);
                    }
                } else {
                    buffer.prepend(0,
                                   &format!("{}:{}:{} - ",
                                            loc.file.name,
                                            cm.doctest_offset_line(loc.line),
                                            loc.col.0 + 1),
                                   Style::LineAndColumn);
                }
            } else if !self.short_message {
                // remember where we are in the output buffer for easy reference
                let buffer_msg_line_offset = buffer.num_lines();

                // Add spacing line
                draw_col_separator(&mut buffer, buffer_msg_line_offset, max_line_num_len + 1);

                // Then, the secondary file indicator
                buffer.prepend(buffer_msg_line_offset + 1, "::: ", Style::LineNumber);
                let loc = if let Some(first_line) = annotated_file.lines.first() {
                    let col = if let Some(first_annotation) = first_line.annotations.first() {
                        format!(":{}", first_annotation.start_col + 1)
                    } else { "".to_string() };
                    format!("{}:{}{}",
                            annotated_file.file.name,
                            cm.doctest_offset_line(first_line.line_index),
                            col)
                } else {
                    annotated_file.file.name.to_string()
                };
                buffer.append(buffer_msg_line_offset + 1,
                              &loc,
                              Style::LineAndColumn);
                for _ in 0..max_line_num_len {
                    buffer.prepend(buffer_msg_line_offset + 1, " ", Style::NoStyle);
                }
            }

            if !self.short_message {
                // Put in the spacer between the location and annotated source
                let buffer_msg_line_offset = buffer.num_lines();
                draw_col_separator_no_space(&mut buffer,
                                            buffer_msg_line_offset,
                                            max_line_num_len + 1);

                // Contains the vertical lines' positions for active multiline annotations
                let mut multilines = HashMap::new();

                // Next, output the annotate source for this file
                for line_idx in 0..annotated_file.lines.len() {
                    let previous_buffer_line = buffer.num_lines();

                    let width_offset = 3 + max_line_num_len;
                    let code_offset = if annotated_file.multiline_depth == 0 {
                        width_offset
                    } else {
                        width_offset + annotated_file.multiline_depth + 1
                    };

                    let depths = self.render_source_line(&mut buffer,
                                                         annotated_file.file.clone(),
                                                         &annotated_file.lines[line_idx],
                                                         width_offset,
                                                         code_offset);

                    let mut to_add = HashMap::new();

                    for (depth, style) in depths {
                        if multilines.get(&depth).is_some() {
                            multilines.remove(&depth);
                        } else {
                            to_add.insert(depth, style);
                        }
                    }

                    // Set the multiline annotation vertical lines to the left of
                    // the code in this line.
                    for (depth, style) in &multilines {
                        for line in previous_buffer_line..buffer.num_lines() {
                            draw_multiline_line(&mut buffer,
                                                line,
                                                width_offset,
                                                *depth,
                                                *style);
                        }
                    }
                    // check to see if we need to print out or elide lines that come between
                    // this annotated line and the next one.
                    if line_idx < (annotated_file.lines.len() - 1) {
                        let line_idx_delta = annotated_file.lines[line_idx + 1].line_index -
                                             annotated_file.lines[line_idx].line_index;
                        if line_idx_delta > 2 {
                            let last_buffer_line_num = buffer.num_lines();
                            buffer.puts(last_buffer_line_num, 0, "...", Style::LineNumber);

                            // Set the multiline annotation vertical lines on `...` bridging line.
                            for (depth, style) in &multilines {
                                draw_multiline_line(&mut buffer,
                                                    last_buffer_line_num,
                                                    width_offset,
                                                    *depth,
                                                    *style);
                            }
                        } else if line_idx_delta == 2 {
                            let unannotated_line = annotated_file.file
                                .get_line(annotated_file.lines[line_idx].line_index)
                                .unwrap_or_else(|| Cow::from(""));

                            let last_buffer_line_num = buffer.num_lines();

                            buffer.puts(last_buffer_line_num,
                                        0,
                                        &(annotated_file.lines[line_idx + 1].line_index - 1)
                                            .to_string(),
                                        Style::LineNumber);
                            draw_col_separator(&mut buffer,
                                               last_buffer_line_num,
                                               1 + max_line_num_len);
                            buffer.puts(last_buffer_line_num,
                                        code_offset,
                                        &unannotated_line,
                                        Style::Quotation);

                            for (depth, style) in &multilines {
                                draw_multiline_line(&mut buffer,
                                                    last_buffer_line_num,
                                                    width_offset,
                                                    *depth,
                                                    *style);
                            }
                        }
                    }

                    multilines.extend(&to_add);
                }
            }
        }

        // final step: take our styled buffer, render it, then output it
        emit_to_destination(&buffer.render(), level, &mut self.dst, self.short_message)?;

        Ok(())

    }
