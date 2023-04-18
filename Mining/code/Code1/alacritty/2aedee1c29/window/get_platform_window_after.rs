    pub fn get_platform_window(identity: &Identity, window_config: &WindowConfig) -> WindowBuilder {
        #[cfg(feature = "x11")]
        let icon = {
            let mut decoder = Decoder::new(Cursor::new(WINDOW_ICON));
            decoder.set_transformations(png::Transformations::normalize_to_color8());
            let mut reader = decoder.read_info().expect("invalid embedded icon");
            let mut buf = vec![0; reader.output_buffer_size()];
            let _ = reader.next_frame(&mut buf);
            Icon::from_rgba(buf, reader.info().width, reader.info().height)
                .expect("invalid embedded icon format")
        };

        let builder = WindowBuilder::new()
            .with_title(&identity.title)
            .with_name(&identity.class.general, &identity.class.instance)
            .with_visible(false)
            .with_transparent(true)
            .with_decorations(window_config.decorations != Decorations::None)
            .with_maximized(window_config.maximized())
            .with_fullscreen(window_config.fullscreen());

        #[cfg(feature = "x11")]
        let builder = builder.with_window_icon(Some(icon));

        #[cfg(feature = "x11")]
        let builder = match window_config.decorations_theme_variant() {
            Some(val) => builder.with_gtk_theme_variant(val.to_string()),
            None => builder,
        };

        #[cfg(feature = "wayland")]
        let builder = match window_config.decorations_theme_variant() {
            Some("light") => builder.with_wayland_csd_theme(Theme::Light),
            // Prefer dark theme by default, since default alacritty theme is dark.
            _ => builder.with_wayland_csd_theme(Theme::Dark),
        };

        builder
    }
