    fn update_working_details(
        &mut self,
        line_buffer: &mut LineBuffer,
        completer: &mut dyn Completer,
        painter: &Painter,
    ) {
        if let Some(event) = self.event.take() {
            // Updating all working parameters from the menu before executing any of the
            // possible event
            let max_width = self.get_values().iter().fold(0, |acc, suggestion| {
                let str_len = suggestion.value.len() + self.default_details.col_padding;
                if str_len > acc {
                    str_len
                } else {
                    acc
                }
            });

            // If no default width is found, then the total screen width is used to estimate
            // the column width based on the default number of columns
            let default_width = if let Some(col_width) = self.default_details.col_width {
                col_width
            } else {
                let col_width = painter.screen_width() / self.default_details.columns;
                col_width as usize
            };

            // Adjusting the working width of the column based the max line width found
            // in the menu values
            if max_width > default_width {
                self.working_details.col_width = max_width;
            } else {
                self.working_details.col_width = default_width;
            };

            // The working columns is adjusted based on possible number of columns
            // that could be fitted in the screen with the calculated column width
            let possible_cols = painter.screen_width() / self.working_details.col_width as u16;
            if possible_cols > self.default_details.columns {
                self.working_details.columns = self.default_details.columns.max(1);
            } else {
                self.working_details.columns = possible_cols;
            }

            // Updating the working rows to display the description
            if self.menu_required_lines(painter.screen_width()) <= painter.remaining_lines() {
                self.working_details.description_rows = self.default_details.description_rows;
                self.show_examples = true;
            } else {
                self.working_details.description_rows = painter
                    .remaining_lines()
                    .saturating_sub(self.default_details.selection_rows + 1)
                    as usize;

                self.show_examples = false;
            }

            match event {
                MenuEvent::Activate(_) => {
                    self.reset_position();
                    self.input = Some(line_buffer.get_buffer().to_string());
                    self.update_values(line_buffer, completer);
                }
                MenuEvent::Deactivate => self.active = false,
                MenuEvent::Edit(_) => {
                    self.reset_position();
                    self.update_values(line_buffer, completer);
                    self.update_examples()
                }
                MenuEvent::NextElement => {
                    self.skipped_rows = 0;
                    self.move_next();
                    self.update_examples();
                }
                MenuEvent::PreviousElement => {
                    self.skipped_rows = 0;
                    self.move_previous();
                    self.update_examples();
                }
                MenuEvent::MoveUp => {
                    if let Some(example_index) = self.example_index {
                        if let Some(index) = example_index.checked_sub(1) {
                            self.example_index = Some(index);
                        } else {
                            self.example_index = Some(self.examples.len().saturating_sub(1));
                        }
                    } else {
                        self.example_index = Some(0);
                    }
                }
                MenuEvent::MoveDown => {
                    if let Some(example_index) = self.example_index {
                        let index = example_index + 1;
                        if index < self.examples.len() {
                            self.example_index = Some(index);
                        } else {
                            self.example_index = Some(0);
                        }
                    } else {
                        self.example_index = Some(0);
                    }
                }
                MenuEvent::MoveLeft => self.skipped_rows = self.skipped_rows.saturating_sub(1),
                MenuEvent::MoveRight => {
                    let skipped = self.skipped_rows + 1;
                    let description_rows = self
                        .get_value()
                        .and_then(|suggestion| suggestion.description)
                        .unwrap_or_else(|| "".to_string())
                        .lines()
                        .count();

                    let allowed_skips =
                        description_rows.saturating_sub(self.working_details.description_rows);

                    if skipped < allowed_skips {
                        self.skipped_rows = skipped;
                    } else {
                        self.skipped_rows = allowed_skips;
                    }
                }
                MenuEvent::PreviousPage | MenuEvent::NextPage => {}
            }
        }
    }
