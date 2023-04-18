    pub fn update_glyph_cache(&mut self, config: &Config, font_size_delta: i8) {
        let cache = &mut self.glyph_cache;
        self.renderer.with_loader(|mut api| {
            let _ = cache.update_font_size(config.font(), font_size_delta, &mut api);
        });

        let metrics = cache.font_metrics();
        self.size_info.cell_width = (metrics.average_advance + config.font().offset().x as f64) as f32;
        self.size_info.cell_height = (metrics.line_height + config.font().offset().y as f64) as f32;
    }
