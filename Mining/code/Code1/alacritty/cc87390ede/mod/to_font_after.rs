    pub fn to_font(&self, size: f64, load_fallbacks:bool) -> Font {
        let ct_font = ct_new_from_descriptor(&self.ct_descriptor, size);
        let cg_font = ct_font.copy_to_CGFont();

        let fallbacks = if load_fallbacks {
            descriptors_for_family("Menlo")
                .into_iter()
                .filter(|d| d.font_name == "Menlo-Regular")
                .nth(0)
                .map(|descriptor| {
                    let menlo = ct_new_from_descriptor(&descriptor.ct_descriptor, size);

                    // TODO fixme, hardcoded en for english
                    let mut fallbacks = cascade_list_for_languages(&menlo, &vec!["en".to_owned()])
                        .into_iter()
                        .filter(|desc| desc.font_path != "")
                        .map(|desc| desc.to_font(size, false))
                        .collect::<Vec<_>>();

                    // TODO, we can't use apple's proposed
                    // .Apple Symbol Fallback (filtered out below),
                    // but not having these makes us not able to render
                    // many chars. We add the symbols back in.
                    // Investigate if we can actually use the .-prefixed
                    // fallbacks somehow.
                    descriptors_for_family("Apple Symbols")
                        .into_iter()
                        .next() // should only have one element; use it
                        .map(|descriptor| {
                            fallbacks.push(descriptor.to_font(size, false))
                        });

                    // Include Menlo in the fallback list as well
                    fallbacks.insert(0, Font {
                        cg_font: menlo.copy_to_CGFont(),
                        ct_font: menlo,
                        fallbacks: Vec::new()
                    });

                    fallbacks
                })
                .unwrap_or_else(Vec::new)
        } else {
            Vec::new()
        };

        Font {
            ct_font: ct_font,
            cg_font: cg_font,
            fallbacks: fallbacks,
        }
    }
