    pub fn config(&self, inputs: &[Input]) -> Result<Config> {
        let style_components = self.style_components()?;

        let paging_mode = match self.matches.value_of("paging") {
            Some("always") => PagingMode::Always,
            Some("never") => PagingMode::Never,
            Some("auto") | None => {
                if self.matches.occurrences_of("plain") > 1 {
                    // If we have -pp as an option when in auto mode, the pager should be disabled.
                    PagingMode::Never
                } else if self.matches.is_present("no-paging") {
                    PagingMode::Never
                } else if inputs.iter().any(Input::is_stdin) {
                    // If we are reading from stdin, only enable paging if we write to an
                    // interactive terminal and if we do not *read* from an interactive
                    // terminal.
                    if self.interactive_output && !atty::is(Stream::Stdin) {
                        PagingMode::QuitIfOneScreen
                    } else {
                        PagingMode::Never
                    }
                } else if self.interactive_output {
                    PagingMode::QuitIfOneScreen
                } else {
                    PagingMode::Never
                }
            }
            _ => unreachable!("other values for --paging are not allowed"),
        };

        let mut syntax_mapping = SyntaxMapping::builtin();

        if let Some(values) = self.matches.values_of("map-syntax") {
            for from_to in values {
                let parts: Vec<_> = from_to.split(':').collect();

                if parts.len() != 2 {
                    return Err("Invalid syntax mapping. The format of the -m/--map-syntax option is '<glob-pattern>:<syntax-name>'. For example: '*.cpp:C++'.".into());
                }

                syntax_mapping.insert(parts[0], MappingTarget::MapTo(parts[1]))?;
            }
        }

        let maybe_term_width = self.matches.value_of("terminal-width").and_then(|w| {
            if w.starts_with('+') || w.starts_with('-') {
                // Treat argument as a delta to the current terminal width
                w.parse().ok().map(|delta: i16| {
                    let old_width: u16 = Term::stdout().size().1;
                    let new_width: i32 = i32::from(old_width) + i32::from(delta);

                    if new_width <= 0 {
                        old_width as usize
                    } else {
                        new_width as usize
                    }
                })
            } else {
                w.parse().ok()
            }
        });

        Ok(Config {
            true_color: is_truecolor_terminal(),
            language: self.matches.value_of("language").or_else(|| {
                if self.matches.is_present("show-all") {
                    Some("show-nonprintable")
                } else {
                    None
                }
            }),
            show_nonprintable: self.matches.is_present("show-all"),
            wrapping_mode: if self.interactive_output || maybe_term_width.is_some() {
                match self.matches.value_of("wrap") {
                    Some("character") => WrappingMode::Character,
                    Some("never") => WrappingMode::NoWrapping,
                    Some("auto") | None => {
                        if style_components.plain() {
                            WrappingMode::NoWrapping
                        } else {
                            WrappingMode::Character
                        }
                    }
                    _ => unreachable!("other values for --paging are not allowed"),
                }
            } else {
                // We don't have the tty width when piping to another program.
                // There's no point in wrapping when this is the case.
                WrappingMode::NoWrapping
            },
            colored_output: self.matches.is_present("force-colorization")
                || match self.matches.value_of("color") {
                    Some("always") => true,
                    Some("never") => false,
                    Some("auto") => env::var_os("NO_COLOR").is_none() && self.interactive_output,
                    _ => unreachable!("other values for --color are not allowed"),
                },
            paging_mode,
            term_width: maybe_term_width.unwrap_or(Term::stdout().size().1 as usize),
            loop_through: !(self.interactive_output
                || self.matches.value_of("color") == Some("always")
                || self.matches.value_of("decorations") == Some("always")
                || self.matches.is_present("force-colorization")),
            tab_width: self
                .matches
                .value_of("tabs")
                .map(String::from)
                .or_else(|| env::var("BAT_TABS").ok())
                .and_then(|t| t.parse().ok())
                .unwrap_or(
                    if style_components.plain() && paging_mode == PagingMode::Never {
                        0
                    } else {
                        4
                    },
                ),
            theme: self
                .matches
                .value_of("theme")
                .map(String::from)
                .or_else(|| env::var("BAT_THEME").ok())
                .map(|s| {
                    if s == "default" {
                        String::from(HighlightingAssets::default_theme())
                    } else {
                        s
                    }
                })
                .unwrap_or_else(|| String::from(HighlightingAssets::default_theme())),
            visible_lines: match self.matches.is_present("diff") {
                #[cfg(feature = "git")]
                true => VisibleLines::DiffContext(
                    self.matches
                        .value_of("diff-context")
                        .and_then(|t| t.parse().ok())
                        .unwrap_or(2),
                ),

                _ => VisibleLines::Ranges(
                    self.matches
                        .values_of("line-range")
                        .map(|vs| vs.map(LineRange::from).collect())
                        .transpose()?
                        .map(LineRanges::from)
                        .unwrap_or_default(),
                ),
            },
            style_components,
            syntax_mapping,
            pager: self.matches.value_of("pager"),
            use_italic_text: self.matches.value_of("italic-text") == Some("always"),
            highlighted_lines: self
                .matches
                .values_of("highlight-line")
                .map(|ws| ws.map(LineRange::from).collect())
                .transpose()?
                .map(LineRanges::from)
                .map(HighlightedLineRanges)
                .unwrap_or_default(),
        })
    }
