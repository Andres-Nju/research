    pub fn load_glyph(
        active_tex: &mut GLuint,
        atlas: &mut Vec<Atlas>,
        current_atlas: &mut usize,
        rasterized: &RasterizedGlyph,
    ) -> Glyph {
        // At least one atlas is guaranteed to be in the `self.atlas` list; thus
        // the unwrap.
        match atlas[*current_atlas].insert(rasterized, active_tex) {
            Ok(glyph) => glyph,
            Err(AtlasInsertError::Full) => {
                // Get the context type before adding a new Atlas.
                let is_gles_context = atlas[*current_atlas].is_gles_context;

                // Advance the current Atlas index.
                *current_atlas += 1;
                if *current_atlas == atlas.len() {
                    let new = Atlas::new(ATLAS_SIZE, is_gles_context);
                    *active_tex = 0; // Atlas::new binds a texture. Ugh this is sloppy.
                    atlas.push(new);
                }
                Atlas::load_glyph(active_tex, atlas, current_atlas, rasterized)
            },
            Err(AtlasInsertError::GlyphTooLarge) => Glyph {
                tex_id: atlas[*current_atlas].id,
                multicolor: false,
                top: 0,
                left: 0,
                width: 0,
                height: 0,
                uv_bot: 0.,
                uv_left: 0.,
                uv_width: 0.,
                uv_height: 0.,
            },
        }
    }
