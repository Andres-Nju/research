    fn metrics(&self, key: FontKey, _size: Size) -> Result<Metrics, Error> {
        let face = self.faces.get(&key).ok_or(Error::FontNotLoaded)?;
        let full = self.full_metrics(key)?;

        let height = (full.size_metrics.height / 64) as f64;
        let descent = (full.size_metrics.descender / 64) as f32;

        // Get underline position and thickness in device pixels
        let x_scale = full.size_metrics.x_scale as f32 / 65536.0;
        let mut underline_position = f32::from(face.ft_face.underline_position()) * x_scale / 64.;
        let mut underline_thickness = f32::from(face.ft_face.underline_thickness()) * x_scale / 64.;

        // Fallback for bitmap fonts which do not provide underline metrics
        if underline_position == 0. {
            underline_thickness = (descent / 5.).round();
            underline_position = descent / 2.;
        }

        // Get strikeout position and thickness in device pixels
        let (strikeout_position, strikeout_thickness) =
            match TrueTypeOS2Table::from_face(&mut face.ft_face.clone()) {
                Some(os2) => {
                    let strikeout_position = f32::from(os2.y_strikeout_position()) * x_scale / 64.;
                    let strikeout_thickness = f32::from(os2.y_strikeout_size()) * x_scale / 64.;
                    (strikeout_position, strikeout_thickness)
                },
                _ => {
                    // Fallback if font doesn't provide info about strikeout
                    trace!("Using fallback strikeout metrics");
                    let strikeout_position = height as f32 / 2. + descent;
                    (strikeout_position, underline_thickness)
                },
            };

        Ok(Metrics {
            average_advance: full.cell_width,
            line_height: height,
            descent,
            underline_position,
            underline_thickness,
            strikeout_position,
            strikeout_thickness,
        })
    }
