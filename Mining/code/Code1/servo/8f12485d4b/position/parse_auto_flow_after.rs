        fn parse_auto_flow<'i, 't>(input: &mut Parser<'i, 't>, is_row: bool)
                                   -> Result<SpecifiedAutoFlow, ParseError<'i>> {
            let mut auto_flow = None;
            let mut dense = false;
            for _ in 0..2 {
                if input.try(|i| i.expect_ident_matching("auto-flow")).is_ok() {
                    auto_flow = if is_row {
                        Some(AutoFlow::Row)
                    } else {
                        Some(AutoFlow::Column)
                    };
                } else if input.try(|i| i.expect_ident_matching("dense")).is_ok() {
                    dense = true;
                } else {
                    break
                }
            }

            auto_flow.map(|flow| {
                SpecifiedAutoFlow {
                    autoflow: flow,
                    dense: dense,
                }
            }).ok_or(StyleParseError::UnspecifiedError.into())
        }

        if let Ok((rows, cols, areas)) = input.try(|i| super::grid_template::parse_grid_template(context, i)) {
            temp_rows = rows;
            temp_cols = cols;
            temp_areas = areas;
        } else if let Ok(rows) = input.try(|i| GridTemplateComponent::parse(context, i)) {
            temp_rows = rows;
            input.expect_delim('/')?;
            flow = parse_auto_flow(input, false)?;
            auto_cols = grid_auto_columns::parse(context, input).unwrap_or_default();
        } else {
            flow = parse_auto_flow(input, true)?;
            auto_rows = input.try(|i| grid_auto_rows::parse(context, i)).unwrap_or_default();
            input.expect_delim('/')?;
            temp_cols = GridTemplateComponent::parse(context, input)?;
        }

        Ok(expanded! {
            grid_template_rows: temp_rows,
            grid_template_columns: temp_cols,
            grid_template_areas: temp_areas,
            grid_auto_rows: auto_rows,
            grid_auto_columns: auto_cols,
            grid_auto_flow: flow,
            // This shorthand also resets grid gap
            grid_row_gap: LengthOrPercentage::zero(),
            grid_column_gap: LengthOrPercentage::zero(),
        })
    }

    impl<'a> LonghandsToSerialize<'a> {
        /// Returns true if other sub properties except template-{rows,columns} are initial.
        fn is_grid_template(&self) -> bool {
            *self.grid_template_areas == Either::Second(None_) &&
            *self.grid_auto_rows == TrackSize::default() &&
            *self.grid_auto_columns == TrackSize::default() &&
            *self.grid_auto_flow == grid_auto_flow::get_initial_value()
        }
    }

    impl<'a> ToCss for LonghandsToSerialize<'a> {
        fn to_css<W>(&self, dest: &mut W) -> fmt::Result where W: fmt::Write {
            if *self.grid_template_areas != Either::Second(None_) ||
               (*self.grid_template_rows != GridTemplateComponent::None &&
                   *self.grid_template_columns != GridTemplateComponent::None) ||
               self.is_grid_template() {
                return super::grid_template::serialize_grid_template(self.grid_template_rows,
                                                                     self.grid_template_columns,
                                                                     self.grid_template_areas, dest);
            }

            if self.grid_auto_flow.autoflow == AutoFlow::Column {
                self.grid_template_rows.to_css(dest)?;
                dest.write_str(" / auto-flow")?;
                if self.grid_auto_flow.dense {
                    dest.write_str(" dense")?;
                }

                if !self.grid_auto_columns.is_default() {
                    dest.write_str(" ")?;
                    self.grid_auto_columns.to_css(dest)?;
                }
            } else {
                dest.write_str("auto-flow")?;
                if self.grid_auto_flow.dense {
                    dest.write_str(" dense")?;
                }

                if !self.grid_auto_rows.is_default() {
                    dest.write_str(" ")?;
                    self.grid_auto_rows.to_css(dest)?;
                }

                dest.write_str(" / ")?;
                self.grid_template_columns.to_css(dest)?;
            }
            Ok(())
        }
    }
</%helpers:shorthand>

<%helpers:shorthand name="place-content" sub_properties="align-content justify-content"
                    spec="https://drafts.csswg.org/css-align/#propdef-place-content"
                    products="gecko" disable_when_testing="True">
    use properties::longhands::align_content;
    use properties::longhands::justify_content;

    pub fn parse_value<'i, 't>(context: &ParserContext, input: &mut Parser<'i, 't>)
                               -> Result<Longhands, ParseError<'i>> {
        let align = align_content::parse(context, input)?;
        if align.has_extra_flags() {
            return Err(StyleParseError::UnspecifiedError.into());
        }
        let justify = input.try(|input| justify_content::parse(context, input))
                           .unwrap_or(justify_content::SpecifiedValue::from(align));
        if justify.has_extra_flags() {
            return Err(StyleParseError::UnspecifiedError.into());
        }

        Ok(expanded! {
            align_content: align,
            justify_content: justify,
        })
    }
