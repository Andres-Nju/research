    fn load_font(&mut self, desc: &FontDesc, _size: Size) -> Result<FontKey, Error> {
        // Fast path if face is already loaded
        if let Some(key) = self.keys.get(desc) {
            return Ok(*key);
        }

        let family = self
            .available_fonts
            .get_font_family_by_name(&desc.name)
            .ok_or_else(|| Error::MissingFont(desc.clone()))?;

        let font = match desc.style {
            Style::Description { weight, slant } => {
                // This searches for the "best" font - should mean we don't have to worry about
                // fallbacks if our exact desired weight/style isn't available
                Ok(family.get_first_matching_font(weight.into(), FontStretch::Normal, slant.into()))
            },
            Style::Specific(ref style) => {
                let mut idx = 0;
                let count = family.get_font_count();

                loop {
                    if idx == count {
                        break Err(Error::MissingFont(desc.clone()));
                    }

                    let font = family.get_font(idx);

                    if font.face_name() == *style {
                        break Ok(font);
                    }

                    idx += 1;
                }
            },
        }?;

        let key = FontKey::next();
        self.keys.insert(desc.clone(), key);
        self.fonts.insert(key, font.into());

        Ok(key)
    }
