pub fn wrap_content(
    cell_width: usize,
    mut input: impl Iterator<Item = Subline>,
    color_hm: &HashMap<String, Style>,
    re_leading: &regex::Regex,
    re_trailing: &regex::Regex,
) -> (Vec<WrappedLine>, usize) {
    let mut lines = vec![];
    let mut current_line: Vec<Subline> = vec![];
    let mut current_width = 0;
    let mut first = true;
    let mut max_width = 0;
    let lead_trail_space_bg_color = color_hm
        .get("leading_trailing_space_bg")
        .unwrap_or(&Style::default())
        .to_owned();

    loop {
        match input.next() {
            Some(item) => {
                if !first {
                    current_width += 1;
                } else {
                    first = false;
                }

                if item.width + current_width > cell_width {
                    // If this is a really long single word, we need to split the word
                    if current_line.len() == 1 && current_width > cell_width {
                        max_width = cell_width;
                        let sublines = split_word(cell_width, &current_line[0].subline);
                        for subline in sublines {
                            let width = subline.width;
                            lines.push(Line {
                                sublines: vec![subline],
                                width,
                            });
                        }

                        first = true;

                        current_width = item.width;
                        current_line = vec![item];
                    } else {
                        if !current_line.is_empty() {
                            lines.push(Line {
                                sublines: current_line,
                                width: current_width,
                            });
                        }

                        first = true;

                        current_width = item.width;
                        current_line = vec![item];
                        max_width = std::cmp::max(max_width, current_width);
                    }
                } else {
                    current_width += item.width;
                    current_line.push(item);
                }
            }
            None => {
                if current_width > cell_width {
                    // We need to break up the last word
                    let sublines = split_word(cell_width, &current_line[0].subline);
                    for subline in sublines {
                        let width = subline.width;
                        lines.push(Line {
                            sublines: vec![subline],
                            width,
                        });
                    }
                } else if current_width > 0 {
                    lines.push(Line {
                        sublines: current_line,
                        width: current_width,
                    });
                }
                break;
            }
        }
    }

    let mut current_max = 0;
    let mut output = vec![];

    for line in lines {
        let mut current_line_width = 0;
        let mut first = true;
        let mut current_line = String::new();

        for subline in line.sublines {
            if !first {
                current_line_width += subline.width;

                if current_line_width + 1 < cell_width {
                    current_line_width += 1;
                    current_line.push(' ');
                }
            } else {
                first = false;
                current_line_width = subline.width;
            }
            current_line.push_str(&subline.subline);
        }

        if current_line_width > current_max {
            current_max = current_line_width;
        }

        // highlight leading and trailing spaces so they stand out.
        let mut bg_color_string = Style::default().prefix().to_string();
        // right now config settings can only set foreground colors so, in this
        // instance we take the foreground color and make it a background color
        if let Some(bg) = lead_trail_space_bg_color.foreground {
            bg_color_string = Style::default().on(bg).prefix().to_string()
        };

        if let Some(leading_match) = re_leading.find(&current_line.clone()) {
            String::insert_str(
                &mut current_line,
                leading_match.end(),
                nu_ansi_term::ansi::RESET,
            );
            String::insert_str(&mut current_line, leading_match.start(), &bg_color_string);
        }

        if let Some(trailing_match) = re_trailing.find(&current_line.clone()) {
            String::insert_str(&mut current_line, trailing_match.start(), &bg_color_string);
            current_line += nu_ansi_term::ansi::RESET;
        }

        output.push(WrappedLine {
            line: current_line,
            width: current_line_width,
        });
    }

    (output, current_max)
}
