    fn metrics(&self) -> FontMetrics {
        let dm = self.face.metrics();

        let au_from_du = |du| -> Au { Au::from_f32_px(du as f32 * self.du_to_px) };
        let au_from_du_s = |du| -> Au { Au:: from_f32_px(du as f32 * self.scaled_du_to_px) };

        // anything that we calculate and don't just pull out of self.face.metrics
        // is pulled out here for clarity
        let leading = dm.ascent - dm.capHeight;

        let metrics = FontMetrics {
            underline_size:   au_from_du(dm.underlineThickness as i32),
            underline_offset: au_from_du_s(dm.underlinePosition as i32),
            strikeout_size:   au_from_du(dm.strikethroughThickness as i32),
            strikeout_offset: au_from_du_s(dm.strikethroughPosition as i32),
            leading:          au_from_du_s(leading as i32),
            x_height:         au_from_du_s(dm.xHeight as i32),
            em_size:          au_from_em(self.em_size as f64),
            ascent:           au_from_du_s(dm.ascent as i32),
            descent:          au_from_du_s(dm.descent as i32),
            max_advance:      au_from_pt(0.0), // FIXME
            average_advance:  au_from_pt(0.0), // FIXME
            line_gap:         au_from_du_s((dm.ascent + dm.descent + dm.lineGap as u16) as i32),
        };
        debug!("Font metrics (@{} pt): {:?}", self.em_size * 12., metrics);
        metrics
    }
