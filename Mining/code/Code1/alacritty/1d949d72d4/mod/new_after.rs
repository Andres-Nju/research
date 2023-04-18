    pub fn new<L>(
        mut rasterizer: Rasterizer,
        config: &Config,
        loader: &mut L
    ) -> Result<GlyphCache, font::Error>
        where L: LoadGlyph
    {
        let font = config.font();
        let size = font.size();
        let glyph_offset = *font.glyph_offset();

        // Load regular font
        let regular_desc = if let Some(ref style) = font.normal.style {
            FontDesc::new(&font.normal.family[..], font::Style::Specific(style.to_owned()))
        } else {
            let style = font::Style::Description {
                slant: font::Slant::Normal,
                weight: font::Weight::Normal
            };
            FontDesc::new(&font.normal.family[..], style)
        };

        let regular = rasterizer
            .load_font(&regular_desc, size)?;

        // Load bold font
        let bold_desc = if let Some(ref style) = font.bold.style {
            FontDesc::new(&font.bold.family[..], font::Style::Specific(style.to_owned()))
        } else {
            let style = font::Style::Description {
                slant: font::Slant::Normal,
                weight: font::Weight::Bold
            };
            FontDesc::new(&font.bold.family[..], style)
        };

        let bold = if bold_desc == regular_desc {
            regular
        } else {
            rasterizer.load_font(&bold_desc, size).unwrap_or_else(|_| regular)
        };

        // Load italic font
        let italic_desc = if let Some(ref style) = font.italic.style {
            FontDesc::new(&font.italic.family[..], font::Style::Specific(style.to_owned()))
        } else {
            let style = font::Style::Description {
                slant: font::Slant::Italic,
                weight: font::Weight::Normal
            };
            FontDesc::new(&font.italic.family[..], style)
        };

        let italic = if italic_desc == regular_desc {
            regular
        } else {
            rasterizer.load_font(&italic_desc, size)
                      .unwrap_or_else(|_| regular)
        };

        // Need to load at least one glyph for the face before calling metrics.
        // The glyph requested here ('m' at the time of writing) has no special
        // meaning.
        rasterizer.get_glyph(&GlyphKey { font_key: regular, c: 'm', size: font.size() })?;
        let metrics = rasterizer.metrics(regular)?;

        let mut cache = GlyphCache {
            cache: HashMap::default(),
            rasterizer: rasterizer,
            font_size: font.size(),
            font_key: regular,
            bold_key: bold,
            italic_key: italic,
            glyph_offset: glyph_offset,
            metrics: metrics
        };

        macro_rules! load_glyphs_for_font {
            ($font:expr) => {
                for i in RangeInclusive::new(32u8, 128u8) {
                    cache.get(&GlyphKey {
                        font_key: $font,
                        c: i as char,
                        size: font.size()
                    }, loader);
                }
            }
        }

        load_glyphs_for_font!(regular);
        load_glyphs_for_font!(bold);
        load_glyphs_for_font!(italic);

        Ok(cache)
    }
