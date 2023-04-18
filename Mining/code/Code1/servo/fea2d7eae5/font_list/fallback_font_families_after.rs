    fn fallback_font_families() -> Vec<FontFamily> {
        let alternatives = [
            ("sans-serif", "Roboto-Regular.ttf"),
            ("Droid Sans", "DroidSans.ttf"),
        ];

        alternatives.iter().filter(|item| {
            Path::new(&Self::font_absolute_path(item.1)).exists()
        }).map(|item| {
            FontFamily {
                name: item.0.into(),
                fonts: vec![Font {
                    filename: item.1.into(),
                    weight: None,
                }]
            }
        }). collect()
    }
