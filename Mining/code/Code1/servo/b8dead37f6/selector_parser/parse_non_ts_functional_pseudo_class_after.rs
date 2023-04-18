    fn parse_non_ts_functional_pseudo_class(&self,
                                            name: Cow<str>,
                                            parser: &mut Parser)
                                            -> Result<NonTSPseudoClass, ()> {
        macro_rules! pseudo_class_string_parse {
            (bare: [$(($css:expr, $name:ident, $gecko_type:tt, $state:tt, $flags:tt),)*],
             string: [$(($s_css:expr, $s_name:ident, $s_gecko_type:tt, $s_state:tt, $s_flags:tt),)*]) => {
                match_ignore_ascii_case! { &name,
                    $($s_css => {
                        let name = String::from(parser.expect_ident_or_string()?).into_boxed_str();
                        NonTSPseudoClass::$s_name(name)
                    }, )*
                    "-moz-any" => {
                        let selectors = parser.parse_comma_separated(|input| {
                            ComplexSelector::parse(self, input)
                        })?;
                        // Selectors inside `:-moz-any` may not include combinators.
                        if selectors.iter().any(|s| s.next.is_some()) {
                            return Err(())
                        }
                        NonTSPseudoClass::MozAny(selectors)
                    }
                    _ => return Err(())
                }
            }
        }
        let pseudo_class = apply_non_ts_list!(pseudo_class_string_parse);
        if !pseudo_class.is_internal() || self.in_user_agent_stylesheet() {
            Ok(pseudo_class)
        } else {
            Err(())
        }
    }
