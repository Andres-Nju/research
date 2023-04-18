    pub fn update_font_size<L: LoadGlyph>(
        &mut self,
        font: &Font,
        scale_factor: f64,
        loader: &mut L,
    ) -> Result<(), crossfont::Error> {
        // Update dpi scaling.
        self.rasterizer.update_dpr(scale_factor as f32);
        self.font_offset = font.offset;

        // Recompute font keys.
        let (regular, bold, italic, bold_italic) =
            Self::compute_font_keys(font, &mut self.rasterizer)?;

        self.rasterizer.get_glyph(GlyphKey {
            font_key: regular,
            character: 'm',
            size: font.size(),
        })?;
        let metrics = self.rasterizer.metrics(regular, font.size())?;

        info!("Font size changed to {:?} with scale factor of {}", font.size(), scale_factor);

        self.font_size = font.size();
        self.font_key = regular;
        self.bold_key = bold;
        self.italic_key = italic;
        self.bold_italic_key = bold_italic;
        self.metrics = metrics;
        self.builtin_box_drawing = font.builtin_box_drawing;

        self.clear_glyph_cache(loader);

        Ok(())
    }
