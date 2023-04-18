    fn run(
        &self,
        engine_state: &nu_protocol::engine::EngineState,
        stack: &mut nu_protocol::engine::Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, ShellError> {
        let list: bool = call.has_flag("list");
        let escape: bool = call.has_flag("escape");
        let osc: bool = call.has_flag("osc");
        let use_ansi_coloring = engine_state.get_config().use_ansi_coloring;

        if list {
            return generate_ansi_code_list(engine_state, call.head, use_ansi_coloring);
        }

        // The code can now be one of the ansi abbreviations like green_bold
        // or it can be a record like this: { fg: "#ff0000" bg: "#00ff00" attr: bli }
        // this record is defined in nu-color-config crate
        let code: Value = match call.opt(engine_state, stack, 0)? {
            Some(c) => c,
            None => return Err(ShellError::MissingParameter("code".into(), call.head)),
        };

        let param_is_string = matches!(code, Value::String { val: _, span: _ });

        if escape && osc {
            return Err(ShellError::IncompatibleParameters {
                left_message: "escape".into(),
                left_span: call
                    .get_named_arg("escape")
                    .expect("Unexpected missing argument")
                    .span,
                right_message: "osc".into(),
                right_span: call
                    .get_named_arg("osc")
                    .expect("Unexpected missing argument")
                    .span,
            });
        }

        let code_string = if param_is_string {
            code.as_string().expect("error getting code as string")
        } else {
            "".to_string()
        };

        let param_is_valid_string = param_is_string && !code_string.is_empty();

        if (escape || osc) && (param_is_valid_string) {
            let code_vec: Vec<char> = code_string.chars().collect();
            if code_vec[0] == '\\' {
                return Err(ShellError::UnsupportedInput(
                    String::from("no need for escape characters"),
                    call.get_flag_expr("escape")
                        .expect("Unexpected missing argument")
                        .span,
                ));
            }
        }

        let output = if escape && param_is_valid_string {
            format!("\x1b[{}", code_string)
        } else if osc && param_is_valid_string {
            // Operating system command aka osc  ESC ] <- note the right brace, not left brace for osc
            // OCS's need to end with a bell '\x07' char
            format!("\x1b]{};", code_string)
        } else if param_is_valid_string {
            // parse hex colors like #00FF00
            if code_string.starts_with('#') {
                match nu_color_config::color_from_hex(&code_string) {
                    Ok(color) => match color {
                        Some(c) => c.prefix().to_string(),
                        None => Color::White.prefix().to_string(),
                    },
                    Err(err) => {
                        return Err(ShellError::GenericError(
                            "error parsing hex color".to_string(),
                            format!("{}", err),
                            Some(code.span()?),
                            None,
                            Vec::new(),
                        ));
                    }
                }
            } else {
                match str_to_ansi(&code_string) {
                    Some(c) => c,
                    None => {
                        return Err(ShellError::UnsupportedInput(
                            String::from("Unknown ansi code"),
                            call.positional_nth(0)
                                .expect("Unexpected missing argument")
                                .span,
                        ))
                    }
                }
            }
        } else {
            // This is a record that should look like
            // { fg: "#ff0000" bg: "#00ff00" attr: bli }
            let record = code.as_record()?;
            // create a NuStyle to parse the information into
            let mut nu_style = nu_color_config::NuStyle {
                fg: None,
                bg: None,
                attr: None,
            };
            // Iterate and populate NuStyle with real values
            for (k, v) in record.0.iter().zip(record.1) {
                match k.as_str() {
                    "fg" => nu_style.fg = Some(v.as_string()?),
                    "bg" => nu_style.bg = Some(v.as_string()?),
                    "attr" => nu_style.attr = Some(v.as_string()?),
                    _ => {
                        return Err(ShellError::IncompatibleParametersSingle(
                            format!("problem with key: {}", k),
                            code.span().expect("error with span"),
                        ))
                    }
                }
            }
            // Now create a nu_ansi_term::Style from the NuStyle
            let style = nu_color_config::parse_nustyle(nu_style);
            // Return the prefix string. The prefix is the Ansi String. The suffix would be 0m, reset/stop coloring.
            style.prefix().to_string()
        };

        Ok(Value::string(output, call.head).into_pipeline_data())
    }
